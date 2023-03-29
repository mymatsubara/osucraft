use bevy_ecs::{prelude::EventReader, query::Added, system::Query};
use valence::{
    client::event::ChatCommand,
    prelude::{Client, Color},
    protocol::{
        packets::s2c::{
            commands::{Node, NodeData, Parser, StringArg},
            play::Commands,
        },
        TextFormat, VarInt,
    },
};

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
                        name: "search-song",
                    },
                    executable: true,
                    redirect_node: None,
                },
                Node {
                    children: vec![],
                    data: NodeData::Argument {
                        name: "song name",
                        parser: Parser::String(StringArg::GreedyPhrase),
                        suggestion: None,
                    },
                    executable: false,
                    redirect_node: None,
                },
                Node {
                    children: vec![],
                    data: NodeData::Literal {
                        name: "reset-search",
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
) {
    for command_event in command_events.iter() {
        let match_client = clients.get_mut(command_event.client);

        match command_event
            .command
            .split_once(' ')
            .map(|(command_name, args)| (command_name, args.replace('"', "")))
            .unwrap_or((command_event.command.as_ref(), String::new()))
        {
            ("search-song", song_name) => {
                dbg!(song_name);
                if let Ok(mut client) = match_client {
                    client.send_message("Not implemented yet".color(Color::RED));
                }
            }
            ("reset-search", _) => {
                if let Ok(mut client) = match_client {
                    client.send_message("Not implmented yet".color(Color::RED));
                }
            }
            _ => {
                if let Ok(mut client) = match_client {
                    client.send_message("Unknown command".color(Color::RED))
                }
            }
        }
    }
}
