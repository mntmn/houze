extern crate sled;
extern crate serde;

use std::io::prelude::*;
use serde::{Serialize, Deserialize};
use std::net::{TcpListener, TcpStream};
use bufstream::BufStream;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Item {
    title: String,
    text: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Room {
    title: String,
    text: String,
    exits: String,
    color_bg: i8,
    size: i8,
    items: Vec<Item>
}

#[derive(Serialize, Deserialize, Debug)]
struct Player {
    id: String, // never changes
    name: String,
    x: i32,
    y: i32,
    z: i32,
    items: Vec<Item>
}

#[derive(Debug)]
struct Context {
    store: sled::Db
}

fn room_key(x: i32, y: i32, z: i32) -> String {
    return format!("room-{}/{}/{}",x,y,z);
}

fn get_obj_with_key<T: serde::de::DeserializeOwned>(ctx: &Context, key: String) -> Option<T> {
    let raw = ctx.store.get(&key);

    // TODO: utf8 error handling
    let obj: Option<T> = match raw {
        Ok(Some(raw)) => {
            match serde_json::from_str(std::str::from_utf8(&raw).unwrap()) {
                Ok(r) => Some(r),
                Err(_e) => {
                    println!("JSON decode error for {} -> {:?}\n", key, raw);
                    None
                }
            }
        },
        Ok(None) => None,
        Err(_e) => {
            println!("Object {} not found\n", key);
            None
        }
    };

    return obj;
}

fn store_obj_with_key<T: serde::Serialize>(ctx: &Context, key: String, obj: T) {
    let raw = serde_json::to_string(&obj).unwrap();
    ctx.store.insert(&key, raw.as_str());
}

fn get_room_at(ctx: &Context, x: i32, y: i32, z: i32) -> Option<Room> {
    let key = room_key(x,y,z);
    return get_obj_with_key(ctx, key);
}

fn store_room_at(ctx: &Context, x: i32, y: i32, z: i32, room: &Room) {
    let key = room_key(x,y,z);
    store_obj_with_key(&ctx, key, room);
}

fn store_player(ctx: &Context, player: &Player) {
    let key = format!("player-{}", player.id);
    store_obj_with_key(&ctx, key, player);
}

fn get_player(ctx: &Context, id: String) -> Option<Player> {
    let key = format!("player-{}", id);
    return get_obj_with_key(ctx, key);
}

fn populate_demo_world(ctx: &Context) {
    let cofmac = Item {
        title: "COFFEE MACHINE".to_string(),
        text: "JUST AN EMPTY COFFEE MACHINE.".to_string()
    };
    let table = Item {
        title: "TABLE".to_string(),
        text: "FUNCTIONAL OFFICE TABLE.".to_string()
    };
    
    let grndcof = Item {
        title: "GROUND COFFEE".to_string(),
        text: "500G OF GROUND COFFEE BEANS.".to_string()
    };
    
    store_room_at(&ctx, 1000,1000,1000, &Room {
        title: "BLAND OFFICE".to_string(),
        text: "A MODERN, GREY OFFICE WITH FUNCTIONAL TABLES AND CHAIRS. SYNTHETIC CARPET.".to_string(),
        exits: "N E".to_string(),
        color_bg: 0,
        size: 5,
        items: [cofmac, table.clone()].to_vec()
    });

    store_room_at(&ctx, 1001,1000,1000, &Room {
        title: "STORAGE CABINET".to_string(),
        text: "A NONDESCRIPT OFFICE STORAGE CABINET.".to_string(),
        exits: "W".to_string(),
        color_bg: 1,
        size: 1,
        items: [grndcof].to_vec()
    });
    
    store_room_at(&ctx, 1000,1001,1000, &Room {
        title: "CORRIDOR".to_string(),
        text: "A NARROW CORRIDOR CONNECTING OFFICE SPACES.".to_string(),
        exits: "N S".to_string(),
        color_bg: 2,
        size: 3,
        items: [].to_vec()
    });
    
    store_room_at(&ctx, 1000,1002,1000, &Room {
        title: "BLAND OFFICE #2".to_string(),
        text: "ANOTHER GREY OFFICE WITH FUNCTIONAL TABLES AND CHAIRS. SYNTHETIC CARPET.".to_string(),
        exits: "S".to_string(),
        color_bg: 3,
        size: 6,
        items: [table.clone()].to_vec()
    });

    match get_player(&ctx, "mntmn".to_string()) {
        None => {
            store_player(&ctx, &Player {
                id: "mntmn".to_string(),
                name: "mntmn".to_string(),
                x: 1000,
                y: 1000,
                z: 1000,
                items: [].to_vec()
            });
        }
        _ => ()
    }
}

fn move_player(ctx: &Context, player: &mut Player, dx: i32, dy: i32, dz: i32) -> bool {
    match get_room_at(ctx, player.x+dx, player.y+dy, player.z+dz) {
        None => {
            false
        }
        Some(_room) => {
            player.x += dx;
            player.y += dy;
            player.z += dz;
            store_player(&ctx, &player);
            true
        }
    }
}

fn handle_client(ctx: &Context, player: &mut Player, mut stream: TcpStream) {
    println!("handle_client!");

    let mut done = false;

    while !done {
        let mut reader = BufStream::new(stream.try_clone().unwrap());
        let mut buf = String::new();
        reader.read_line(&mut buf);
        
        println!("Request: {}", buf);
        println!("Player position: {} {} {}", player.x,player.y,player.z);

        let mut cur_room = get_room_at(&ctx, player.x,player.y,player.z).unwrap();
        let ok_res = ".OK\n".to_string();
        let error_dir = "!DIR ERROR\n".to_string();

        let response = match [buf.chars().nth(0),buf.chars().nth(1)] {
            [Some('T'),Some('R')] => {
                format!("{}\n", cur_room.title)
            }
            [Some('A'),Some('R')] => {
                format!("{}{}{}\n", cur_room.items.len(), cur_room.size, cur_room.color_bg)
            }
            [Some('D'),Some('R')] => {
                format!("{}\n", cur_room.text)
            }
            [Some('T'),Some('I')] => {
                let idx = buf.chars().nth(2).unwrap().to_digit(10).unwrap();
                format!("{}\n", cur_room.items[idx as usize].title.clone())
            }
            [Some('P'),Some('I')] => {
                // pick up item
                let idx = buf.chars().nth(2).unwrap().to_digit(10).unwrap();
                if idx as usize >= cur_room.items.len() {
                    format!("!ITEM ERROR\n")
                } else {
                    let item = cur_room.items[idx as usize].clone();
                    let mut i = 0;
                    cur_room.items.retain(|_| (i != idx, i += 1).0);
                    player.items.push(item);

                    store_room_at(&ctx, player.x,player.y,player.z, &cur_room);
                    store_player(&ctx, &player);

                    format!(".OK\n")
                }
            }
            [Some('G'),Some(dir)] => {
                match dir {
                    'N' => if move_player(ctx, player, 0, 1,0) { ok_res } else { error_dir }
                    'S' => if move_player(ctx, player, 0,-1,0) { ok_res } else { error_dir }
                    'W' => if move_player(ctx, player,  1,0,0) { ok_res } else { error_dir }
                    'E' => if move_player(ctx, player, -1,0,0) { ok_res } else { error_dir }
                    _ => error_dir
                }
            }
            _ => "!SYNTAX ERROR\n".to_string()
        };

        match stream.write(response.as_bytes()) {
            Err(_) => {
                done = true;
            }
            _ => ()
        }
        stream.flush().unwrap();
    }
}

fn main() {
    let ctx = Context {
        store: sled::open("./houze.sled").expect("open")
    };
    
    println!("HOUZE.\n");

    populate_demo_world(&ctx);

    let mut player = get_player(&ctx, "mntmn".to_string()).unwrap();

    let listener = TcpListener::bind("127.0.0.1:25232").unwrap();
    for stream in listener.incoming() {
        handle_client(&ctx, &mut player, stream.unwrap());
    }
}
