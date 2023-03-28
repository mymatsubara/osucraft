use anyhow::{anyhow, Result};
use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};
use valence::{
    client::event::ClickContainer,
    nbt::{compound, List},
    prelude::{Client, Color, Inventory, InventoryKind, OpenInventory},
    protocol::{ItemKind, ItemStack, TextFormat},
};

use bevy_ecs::{
    prelude::{Component, Entity, EventReader},
    query::{Changed, With},
    system::{Commands, Query, ResMut},
};
use osu_file_parser::{Decimal, OsuFile};
use tracing::error;

use crate::{
    inventory::{open_new_inventory, InventoriesToOpen},
    osu::{Osu, OsuStateChange},
    song_selection::{self, SongSelectionInventory},
};

const SONG_SELECTION_SLOT: u16 = 45;
const LAST_SLOT: u16 = 53;

#[derive(Component, Default)]
pub struct BeatmapSelectionInventory {
    beatmaps: Vec<BeatmapFile>,
}

pub struct BeatmapFile {
    osu_file: OsuFile,
    path: PathBuf,
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

    pub fn load_beatmap_dir(&mut self, dir: &PathBuf) -> Result<&Vec<BeatmapFile>> {
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
            .filter_map(|osu_file_path| {
                Some(BeatmapFile {
                    osu_file: read_to_string(&osu_file_path)
                        .ok()?
                        .parse::<OsuFile>()
                        .ok()?,
                    path: osu_file_path,
                })
            })
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

impl BeatmapFile {
    pub fn osu_file(&self) -> &OsuFile {
        &self.osu_file
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
            let Some(metadata) = beatmap.osu_file.metadata.clone() else { continue };
            let Some(difficulty) = beatmap.osu_file.difficulty.clone() else {continue};

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

pub fn handle_beatmap_selection_clicks(
    mut commands: Commands,
    mut beatmap_selections: Query<&mut BeatmapSelectionInventory, With<Inventory>>,
    song_selections: Query<Entity, (With<SongSelectionInventory>, With<Inventory>)>,
    open_inventories: Query<&OpenInventory, With<Client>>,
    mut clients: Query<&mut Client>,
    mut osu: ResMut<Osu>,
    mut inventories_to_open: ResMut<InventoriesToOpen>,
    mut click_events: EventReader<ClickContainer>,
) {
    for click in click_events.iter() {
        // Check if the click occured on a beatmap selection
        if let Ok(beatmap_selection) = open_inventories
            .get(click.client)
            .and_then(|open_inventory| beatmap_selections.get_mut(open_inventory.entity()))
        {
            let slot = click.slot_id.unsigned_abs();
            // Go back to song selection
            if slot == SONG_SELECTION_SLOT {
                for song_selection in song_selections.iter().take(1) {
                    open_new_inventory(
                        &mut commands,
                        click.client,
                        &mut inventories_to_open,
                        song_selection,
                    );

                    if let Err(error) =
                        osu.change_state(OsuStateChange::SongSelection, &mut clients)
                    {
                        error!(
                            "Error while changing to Song Selection state while on beatmap selection: '{}'",
                            error
                        );
                    }
                }
            } else if let Some(selected_beatmap) = beatmap_selection.beatmaps.get(slot as usize) {
                // Close beatmap selection
                commands.entity(click.client).remove::<OpenInventory>();

                // Play map
                if let Err(error) = osu.change_state(
                    OsuStateChange::PrePlaying {
                        beatmap_path: selected_beatmap.path.clone(),
                    },
                    &mut clients,
                ) {
                    error!(
                        "Error while changing to Playing state while on beatmap selection: '{}'",
                        error
                    );
                }
            }
        }
    }
}
