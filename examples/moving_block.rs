use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::info;
use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState {
            player_list: None,
            blocks: vec![],
            default_world: WorldId::NULL,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    blocks: Vec<EntityId>,
    default_world: WorldId,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -25);

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
    type InventoryState = ();

    async fn server_list_ping(
        &self,
        _server: &SharedServer<Self>,
        _remote_addr: SocketAddr,
        _protocol_version: i32,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            player_sample: Cow::Borrowed(&[]),
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: None,
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state.default_world = world_id;

        server.state.player_list = Some(server.player_lists.insert(()).0);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], UnloadedChunk::default(), ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);

        server.state.blocks.extend((0..1).map(|_| {
            let (id, e) = server.entities.insert(EntityKind::FallingBlock, ());
            e.set_world(world_id);
            e.set_position([
                SPAWN_POS.x as f64 + 0.5,
                SPAWN_POS.y as f64 + 5.0,
                SPAWN_POS.z as f64 + 0.5,
            ]);
            if let TrackedData::FallingBlock(falling_block) = e.data_mut() {
                falling_block.set_no_gravity(true)
            }
            id
        }));

        info!("Server is running on: 127.0.0.1");
    }

    fn update(&self, server: &mut Server<Self>) {
        self.handle_connection(server);
        let current_tick = server.current_tick();

        let entity = &mut server.entities[*server.state.blocks.first().unwrap()];
        let tick = current_tick % 64;

        let speed = 16.0;
        let (x, z) = match tick / 16 {
            0 => (speed, 0.0),
            1 => (0.0, speed),
            2 => (-speed, 0.0),
            3 => (0.0, -speed),
            _ => panic!("unreachable"),
        };
        entity.set_velocity([x, 0.0, z]);
    }
}

impl Game {
    fn handle_connection(&self, server: &mut Server<Self>) {
        server.clients.retain(|_, client| {
            if client.created_this_tick() {
                if self
                    .player_count
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                        (count < MAX_PLAYERS).then_some(count + 1)
                    })
                    .is_err()
                {
                    client.disconnect("The server is full!".color(Color::RED));
                    return false;
                }

                match server
                    .entities
                    .insert_with_uuid(EntityKind::Player, client.uuid(), ())
                {
                    Some((id, entity)) => {
                        entity.set_world(server.state.default_world);
                        client.entity_id = id
                    }
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn(server.state.default_world);
                client.set_flat(true);
                client.set_game_mode(GameMode::Creative);
                client.teleport(
                    [
                        SPAWN_POS.x as f64 + 0.5,
                        SPAWN_POS.y as f64 + 1.0,
                        SPAWN_POS.z as f64 + 0.5,
                    ],
                    0.0,
                    0.0,
                );
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                        true,
                    );
                }
            }

            let entity = &mut server.entities[client.entity_id];

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].remove(client.uuid());
                }
                entity.set_deleted(true);

                return false;
            }

            while let Some(event) = client.next_event() {
                event.handle_default(client, entity);
            }

            true
        });
    }
}
