use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use valence::prelude::*;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState {
            player_list: None,
            herobrine: EntityId::NULL,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    herobrine: EntityId,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: usize = 100;
const SIZE_Z: usize = 100;

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension {
            fixed_time: Some(6000),
            ..Dimension::default()
        }]
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
            player_sample: Default::default(),
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/logo-64x64.png").as_slice().into()),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state.player_list = Some(server.player_lists.insert(()).0);

        // initialize chunks
        for chunk_z in -2..Integer::div_ceil(&(SIZE_Z as i32), &16) + 2 {
            for chunk_x in -2..Integer::div_ceil(&(SIZE_X as i32), &16) + 2 {
                world.chunks.insert(
                    [chunk_x as i32, chunk_z as i32],
                    UnloadedChunk::default(),
                    (),
                );
            }
        }

        // initialize blocks in the chunks
        for chunk_x in 0..Integer::div_ceil(&SIZE_X, &16) {
            for chunk_z in 0..Integer::div_ceil(&SIZE_Z, &16) {
                let chunk = world
                    .chunks
                    .get_mut((chunk_x as i32, chunk_z as i32))
                    .unwrap();
                for x in 0..16 {
                    for z in 0..16 {
                        let cell_x = chunk_x * 16 + x;
                        let cell_z = chunk_z * 16 + z;

                        if cell_x < SIZE_X && cell_z < SIZE_Z {
                            chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
                        }
                    }
                }
            }
        }

        let (id, e) = server
            .entities
            .insert_with_uuid(
                EntityKind::Player,
                valence::uuid::uuid!("f84c6a79-0a4e-45e0-879b-cd49ebd4c4e2"),
                (),
            )
            .unwrap();
        server.state.herobrine = id;
        e.set_world(world_id);
        e.set_position(Vec3::new(50., 0., 40.));
        e.set_head_yaw(-180.0);
        //e.set_yaw(yaw as f32);
        //e.set_pitch(pitch as f32);

        server
            .player_lists
            .get_mut(&server.state.player_list.as_ref().unwrap())
            .insert(
                valence::uuid::uuid!("f84c6a79-0a4e-45e0-879b-cd49ebd4c4e2"),
                "Herobrine",
                None,
                GameMode::Survival,
                0,
                Text::text("???"),
            );
    }

    fn update(&self, server: &mut Server<Self>) {
        let time = server.shared.current_tick() as f64 / server.shared.tick_rate() as f64;
        let (world_id, _world) = server.worlds.iter_mut().next().unwrap();
        
        let spawn_pos = [SIZE_X as f64 / 2.0, 1.0, SIZE_Z as f64 / 2.0];

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
                    Some((id, _)) => client.state.entity_id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.spawn(world_id);
                client.set_flat(true);
                client.teleport(spawn_pos, 0.0, 0.0);
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.entity_id);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                return false;
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            if client.position().y <= -20.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    _ => {}
                }
            }

            true
        });

        let herobrine = server.entities.get_mut(server.state.herobrine).expect("missing ???");

        
        
        //herobrine.set_head_yaw(-180.0 + (time * 10ma.0) as f32);
        //herobrine.set_yaw(180.0);

        /* if time > 5.0 {
            let time = time - 5.0;
            let yaw = ((time*4.0).floor() * 5.0) as f32;
            if herobrine.yaw() != yaw {
                herobrine.set_yaw(yaw);
                server.clients.iter_mut().for_each(|c| c.1.send_message(format!("yaw = {}", yaw)));
            }
        } */

    }
}

struct Herobrine {

}

impl Herobrine {
    fn closest_player() {
        
    }
}
