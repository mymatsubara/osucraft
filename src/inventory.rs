use std::mem;

use bevy_ecs::{
    prelude::Entity,
    system::{Commands, ResMut, Resource},
};
use valence::prelude::OpenInventory;

#[derive(Resource, Default)]
pub struct InventoriesToOpen {
    inventories: Vec<InventoryToOpen>,
}

pub struct InventoryToOpen {
    pub client: Entity,
    pub open_inventory: OpenInventory,
}

impl InventoriesToOpen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, inventory: InventoryToOpen) {
        self.inventories.push(inventory);
    }
}

pub fn open_queued_inventories(mut commands: Commands, mut to_open: ResMut<InventoriesToOpen>) {
    let mut inventories_to_open = Vec::new();
    mem::swap(&mut inventories_to_open, &mut to_open.inventories);

    for inventory in inventories_to_open.into_iter() {
        if let Some(mut client) = commands.get_entity(inventory.client) {
            client.insert(inventory.open_inventory);
        }
    }
}
