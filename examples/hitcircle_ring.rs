use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use osucraft::hitcircle::HitcircleRing;
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
            hitcircle_ring: None,
            default_world: WorldId::NULL,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    hitcircle_ring: Option<HitcircleRing>,
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

    fn update(&self, server: &mut Server<Self>) {
        self.handle_connection(server);

        let ring = server.state.hitcircle_ring.take();
        server.state.hitcircle_ring = match ring {
            None => Some(
                HitcircleRing::new(
                    [
                        SPAWN_POS.x as f64,
                        SPAWN_POS.y as f64,
                        SPAWN_POS.z as f64 + 30.0,
                    ],
                    20.0,
                    10.0,
                    server,
                    server.state.default_world,
                )
                .unwrap(),
            ),
            Some(mut ring) => {
                if !ring.tick(server) {
                    None
                } else {
                    Some(ring)
                }
            }
        };
    }

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

        info!("Server is running on: 127.0.0.1");
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
