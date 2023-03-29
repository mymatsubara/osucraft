use anyhow::anyhow;
use bevy_ecs::{
    prelude::EventReader,
    query::{Added, With},
    system::Query,
};
use valence::{
    client::event::ChatCommand,
    prelude::{Client, Color, Inventory},
    protocol::{
        packets::s2c::{
            commands::{Node, NodeData, Parser, StringArg},
            play::Commands,
        },
        TextFormat, VarInt,
    },
};

use crate::song_selection::SongSelectionInventory;

pub fn register_mc_commands(mut new_clients: Query<&mut Client, Added<Client>>) {
    for mut client in &mut new_clients {
        client.write_packet(&Commands {
            commands: vec![
                Node {
                    children: vec![VarInt(1), VarInt(3)],
                    data: NodeData::Root,
                    executable: false,
                    redirect_node: None,
                },
                Node {
                    children: vec![VarInt(2)],
                    data: NodeData::Literal {
                        name: "filter-songs",
                    },
                    executable: true,
                    redirect_node: None,
                },
                Node {
                    children: vec![],
                    data: NodeData::Argument {
                        name: "keywords",
                        parser: Parser::String(StringArg::GreedyPhrase),
                        suggestion: None,
                    },
                    executable: false,
                    redirect_node: None,
                },
                Node {
                    children: vec![],
                    data: NodeData::Literal {
                        name: "reset-filter",
                    },
                    executable: true,
                    redirect_node: None,
                },
            ],
            root_index: VarInt(0),
        });
    }
}

pub fn execute_commands(
    mut clients: Query<&mut Client>,
    mut command_events: EventReader<ChatCommand>,
    mut song_selections: Query<&mut SongSelectionInventory, With<Inventory>>,
) {
    for command_event in command_events.iter() {
        let match_client = clients.get_mut(command_event.client);

        let result = match command_event
            .command
            .split_once(' ')
            .map(|(command_name, args)| (command_name, args.replace('"', "")))
            .unwrap_or((command_event.command.as_ref(), String::new()))
        {
            ("filter-songs", keywords) => {
                if let Ok(mut song_selection) = song_selections.get_single_mut() {
                    song_selection.set_filter(Some(keywords.as_str())).map(|_| {
                        "Songs selection filtered by the keywords: ".color(Color::YELLOW)
                            + format!("'{}'", keywords).color(Color::GREEN)
                    })
                } else {
                    Err(anyhow!("Song selection not found"))
                }
            }
            ("reset-filter", _) => {
                if let Ok(mut song_selection) = song_selections.get_single_mut() {
                    song_selection.set_filter(None).map(|_| {
                        "Song filter reset ".color(Color::YELLOW) + "succefully".color(Color::GREEN)
                    })
                } else {
                    Err(anyhow!("Song selection not found"))
                }
            }
            (command_name, _) => Err(anyhow!("Unknown command: '{}'", command_name)),
        };

        // Send command result to client
        match (result, match_client) {
            (Ok(message), Ok(mut client)) => {
                client.send_message(message);
            }
            (Err(error), Ok(mut client)) => {
                client.send_message(
                    format!("Error occurred while executing the command: '{}'", error)
                        .color(Color::RED),
                );
            }
            _ => (),
        }
    }
}
