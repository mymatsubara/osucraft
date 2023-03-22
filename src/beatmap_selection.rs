use anyhow::{anyhow, Result};
use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};
use valence::{
    nbt::{compound, List},
    prelude::{Color, Inventory, InventoryKind},
    protocol::{ItemKind, ItemStack, TextFormat},
};

use bevy_ecs::{prelude::Component, query::Changed, system::Query};
use osu_file_parser::{
    metadata::{Artist, Title, Version},
    Decimal, OsuFile,
};

use crate::{beatmap::OverallDifficulty, song_selection};

const SONG_SELECTION_SLOT: u16 = 45;
const LAST_SLOT: u16 = 53;

#[derive(Component, Default)]
pub struct BeatmapSelectionInventory {
    beatmaps: Vec<OsuFile>,
}

impl BeatmapSelectionInventory {
    pub fn new() -> (Self, Inventory) {
        (
            Self::default(),
            Inventory::with_title(
                InventoryKind::Generic9x6,
                "Beatmaps".color(Color::DARK_BLUE),
            ),
        )
    }

    pub fn load_beatmap_dir(&mut self, dir: &PathBuf) -> Result<&Vec<OsuFile>> {
        let beatmaps: Vec<_> = read_dir(dir)?
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();

                if let Some(extension) = path.extension() {
                    if extension == "osu" {
                        return Some(path);
                    }
                }

                None
            })
            .filter_map(|osu_file_path| read_to_string(osu_file_path).ok()?.parse::<OsuFile>().ok())
            .collect();

        if beatmaps.is_empty() {
            Err(anyhow!(
                "No beatmap found in directory: '{}'",
                dir.display()
            ))
        } else {
            self.beatmaps = beatmaps;
            Ok(&self.beatmaps)
        }
    }
}

pub fn update_beatmap_selection_inventory(
    mut beatmap_selections: Query<
        (&BeatmapSelectionInventory, &mut Inventory),
        Changed<BeatmapSelectionInventory>,
    >,
) {
    for (beatmap_selection, mut inventory) in &mut beatmap_selections {
        // Clear inventory
        for slot in 0..=LAST_SLOT {
            inventory.replace_slot(slot, None);
        }

        // Set inventories slots
        for (slot, beatmap) in beatmap_selection.beatmaps.iter().enumerate() {
            let Some(metadata) = beatmap.metadata.clone() else { continue };
            let Some(difficulty) = beatmap.difficulty.clone() else {continue};

            let title: String = metadata
                .title
                .map(|title| title.into())
                .unwrap_or("Not named".to_string());
            let difficulty_name: String = metadata
                .version
                .map(|version| version.into())
                .unwrap_or("Not named".to_string());
            let artist: String = metadata
                .artist
                .map(|artist| artist.into())
                .unwrap_or("Not named".to_string());
            let od: String = difficulty
                .overall_difficulty
                .map(|this| {
                    let decimal: Decimal = this.into();
                    decimal.to_string()
                })
                .unwrap_or("Not defined".to_string());
            let ar: String = difficulty
                .approach_rate
                .map(|this| {
                    let decimal: Decimal = this.into();
                    decimal.to_string()
                })
                .unwrap_or("Not defined".to_string());
            let cs: String = difficulty
                .circle_size
                .map(|this| {
                    let decimal: Decimal = this.into();
                    decimal.to_string()
                })
                .unwrap_or("Not defined".to_string());
            let hp: String = difficulty
                .hp_drain_rate
                .map(|this| {
                    let decimal: Decimal = this.into();
                    decimal.to_string()
                })
                .unwrap_or("Not defined".to_string());

            let item = ItemStack::new(
                ItemKind::Map,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => format!(r#"{{"text": "{title} [{difficulty_name}]", "color": "gold"}}"#),
                        "Lore" => List::String(vec![
                            format!(r#"{{"text": "Artist: {artist}", "color": "gray"}}"#),
                            format!(r#"{{"text": ""}}"#),
                            format!(r#"{{"text": "======= Difficulty =======", "color": "gray"}}"#),
                            format!(r#"{{"text": "AR: {ar}   OD: {od}   HP: {hp}   CS: {cs}", "color": "gray"}}"#),
                        ])
                    }
                }),
            );

            inventory.replace_slot(slot as u16, Some(item));
        }

        // Set song selection slot
        let item = ItemStack::new(
            song_selection::SONG_ITEM_KIND,
            1,
            Some(compound! {
                "display" => compound! {
                    "Name" => r#"{"text": "<- (Return to song selection)", "color": "red"}"#
                }
            }),
        );
        inventory.replace_slot(SONG_SELECTION_SLOT, Some(item));
    }
}
