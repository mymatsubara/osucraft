use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::{cmp::min, fs::read_dir, path::PathBuf};

use bevy_ecs::{
    prelude::{Component, Entity, EventReader},
    query::{Changed, With},
    system::{Commands, Query},
};
use valence::{
    client::event::ClickContainer,
    nbt::{compound, List},
    prelude::{Client, Color, Inventory, InventoryKind, OpenInventory},
    protocol::{
        packets::s2c::play::SetContainerContentEncode, ItemKind, ItemStack, TextFormat, VarInt,
    },
};

const SONG_ITEM_KIND: ItemKind = ItemKind::Jukebox;
const ARROW_ITEM_KIND: ItemKind = ItemKind::SpectralArrow;
const PREVIOUS_PAGE_SLOT: u16 = 45;
const NEXT_PAGE_SLOT: u16 = 53;
const PAGE_SIZE: usize = 36;

#[derive(Component)]
pub struct SongSelectionInventory {
    cur_page: usize,
    songs: Vec<PathBuf>,
    reopen_in_clients: Vec<Entity>,
}

struct Song {
    name: String,
    artist: String,
}

impl SongSelectionInventory {
    pub fn new() -> Result<(Self, Inventory)> {
        let inventory =
            Inventory::with_title(InventoryKind::Generic9x6, "Songs".color(Color::DARK_BLUE));

        Ok((
            Self {
                cur_page: 0,
                songs: Self::get_beatmaps()?,
                reopen_in_clients: vec![],
            },
            inventory,
        ))
    }

    pub fn go_to_next_page(&mut self) {
        self.cur_page += 1;
    }

    pub fn go_to_previous_page(&mut self) {
        self.cur_page -= 1;
    }

    fn page_songs(&self) -> Vec<Song> {
        self.page_song_paths()
            .iter()
            .filter_map(|song_path| song_path.file_name().and_then(|f| f.to_str()))
            .filter_map(|filename| Some(filename.split_once(' ')?.1.replace("[no video]", "")))
            .filter_map(|filename| {
                let (artist, name) = filename.split_once(" - ")?;
                Some(Song {
                    artist: artist.to_string(),
                    name: name.to_string(),
                })
            })
            .collect()
    }

    fn page_song_paths(&self) -> &[PathBuf] {
        let start_idx = self.cur_page * PAGE_SIZE;
        let end_idx = min(start_idx + PAGE_SIZE, self.songs.len());
        &self.songs[start_idx..end_idx]
    }

    fn has_next_page(&self) -> bool {
        self.cur_page < self.max_page()
    }

    fn has_previous_page(&self) -> bool {
        self.cur_page != 0
    }

    fn max_page(&self) -> usize {
        (self.songs.len() - 1) / PAGE_SIZE
    }

    fn get_beatmaps() -> Result<Vec<PathBuf>> {
        Ok(read_dir(Self::get_beatmaps_dir()?)?
            .filter_map(|result| result.ok())
            .map(|entry| entry.path())
            .filter(|entry| entry.is_dir() && entry.file_name().is_some())
            .collect::<Vec<_>>())
    }

    fn get_beatmaps_dir() -> Result<PathBuf> {
        let base_dirs = BaseDirs::new().ok_or(anyhow!("No home directory found in the system"))?;
        let beatmaps_dir = base_dirs.data_local_dir().join("osu!").join("Songs");

        if beatmaps_dir.exists() {
            Ok(beatmaps_dir)
        } else {
            Err(anyhow!(
                "Could not find osu song directory: '{}'",
                beatmaps_dir.display()
            ))
        }
    }

    pub fn refresh(&mut self, commands: &mut Commands, client: Entity) {
        self.reopen_in_clients.push(client);
        commands.entity(client).remove::<OpenInventory>();
    }
}

pub fn update_song_selection_inventory(
    mut inventories: Query<
        (&SongSelectionInventory, &mut Inventory),
        Changed<SongSelectionInventory>,
    >,
) {
    for (song_selection, mut inventory) in &mut inventories {
        let max_page = song_selection.max_page() + 1;
        let cur_page = song_selection.cur_page + 1;
        let next_page = cur_page + 1;
        let prev_page = cur_page - 1;

        // Clear inventory
        for slot in 0_u16..=NEXT_PAGE_SLOT {
            inventory.replace_slot(slot, None);
        }

        // Populate page with songs
        for (slot, song) in song_selection.page_songs().iter().enumerate() {
            let item = ItemStack::new(
                SONG_ITEM_KIND,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => format!(r#"{{"text": "{}","color": "gold"}}"#, song.name),
                        "Lore" => List::String(vec![format!(r#"{{"text": "Artist: {}","color": "gray"}}"#, song.artist)])
                    }
                }),
            );

            inventory.replace_slot(slot as u16, Some(item));
        }

        // Add next page button
        if song_selection.has_next_page() {
            let item = ItemStack::new(
                ARROW_ITEM_KIND,
                1,
                Some(compound! {"display" => compound! {
                "Name" => format!(r#"{{"text": "Next page","color": "green"}}"#),
                "Lore" => List::String(vec![format!(r#"{{"text": "Go to page {} of {}","color": "gray"}}"#, next_page, max_page)]),
                }}),
            );
            inventory.replace_slot(NEXT_PAGE_SLOT, Some(item));
        }

        // Add previuos page button
        if song_selection.has_previous_page() {
            let item = ItemStack::new(
                ARROW_ITEM_KIND,
                1,
                Some(compound! {"display" => compound! {
                    "Name" => format!(r#"{{"text": "Previous page","color": "red"}}"#),
                    "Lore" => List::String(vec![format!(r#"{{"text": "Go to page {} of {}","color": "gray"}}"#, prev_page, max_page)]),
                }}),
            );
            inventory.replace_slot(PREVIOUS_PAGE_SLOT, Some(item));
        }
    }
}

pub fn handle_song_selection_clicks(
    mut commands: Commands,
    open_inventories: Query<(Entity, &OpenInventory), With<Client>>,
    mut song_selections: Query<&mut SongSelectionInventory>,
    mut clicks: EventReader<ClickContainer>,
) {
    for click in clicks.iter() {
        if let Some(mut song_selection) = open_inventories
            .iter()
            .find(|(client_entity, _)| *client_entity == click.client)
            .and_then(|(client_entity, inventory)| song_selections.get_mut(inventory.entity()).ok())
        {
            // Clicked next page
            if click.slot_id as u16 == NEXT_PAGE_SLOT && song_selection.has_next_page() {
                song_selection.go_to_next_page();
                song_selection.refresh(&mut commands, click.client);
            }
            // Clicked previous page
            else if click.slot_id as u16 == PREVIOUS_PAGE_SLOT
                && song_selection.has_previous_page()
            {
                song_selection.go_to_previous_page();
                song_selection.refresh(&mut commands, click.client);
            }
            if let Some(selected_song) = song_selection
                .page_song_paths()
                .get(click.slot_id.unsigned_abs() as usize)
            {
                // Update beatmap selection inventory
                todo!()
            }
        }
    }
}

pub fn reopen_inventory(
    mut commands: Commands,
    mut song_selections: Query<(Entity, &mut SongSelectionInventory)>,
) {
    for (inventory, mut song_selection) in &mut song_selections {
        for &client in song_selection.reopen_in_clients.iter() {
            commands
                .entity(client)
                .insert(OpenInventory::new(inventory));
        }

        song_selection.reopen_in_clients.clear();
    }
}
