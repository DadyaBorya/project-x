use std::error::Error;
use std::io;
use std::io::Write;
use std::path::Path;
use coffee_ldr::loader::beacon_pack::BeaconPack;
use coffee_ldr::loader::Coffee;
use comfy_table::{ContentArrangement, Table};

use services::file_service;
use crate::models::config_file::ConfigFile;
use crate::models::payload::Payload;
use crate::models::state::State;

mod services;
mod models;

static CONFIGS_PATH: &str = r"SA";

fn main() -> Result<(), Box<dyn Error>> {
    init()?;

    main_loop()?;

    Ok(())
}

fn init() -> Result<(), Box<dyn Error>> {
    if !file_service::is_dir_exists(CONFIGS_PATH) {
        return Err(format!("Can't find configs folder in {}", CONFIGS_PATH).into());
    }

    let count_dirs = file_service::count_dirs(CONFIGS_PATH)?;
    println!("Found {} config files", count_dirs);
    println!("Write \"help\" to get all commands");
    println!();

    return Ok(());
}

fn main_loop() -> Result<(), Box<dyn Error>> {
    let mut state = State::default();

    loop {
        let input = get_user_input()?;

        match input.as_str() {
            "help" => print_help(),
            "clear" => clean_screen(),
            "list" => print_list(&mut state),
            cmd if cmd.starts_with("search ") => print_search(cmd, &mut state),
            cmd if cmd.starts_with("use ") => handle_use_command(cmd, &mut state),
            "options" => print_options(&mut state),
            "cur" => print_current(&mut state),
            cmd if cmd.contains("run") => handle_run_command(cmd, &mut state),
            "exit" => break,
            str => print_unknown_command(str)
        }
    }

    println!("See u later!");
    Ok(())
}

fn handle_run_command(input: &str, state: &mut State) {
    if state.current_name.is_none() {
        println!("Now no config file selected");
        return;
    }

    let config_files = file_service::get_all_config_files(CONFIGS_PATH);

    if config_files.is_err() {
        println!("Can't get config files");
        return;
    }
    let tokens: Vec<&str> = if input.len() > 3 {
        input[4..].split_whitespace().collect()
    } else { vec![] };

    if &tokens.len() < &state.payloads.iter().filter(|payload| !payload.arg.optional).count() {
        println!("Not enough args. Write \"options\" to get all args");
        return;
    }


    for (i, payload) in state.payloads.iter_mut().enumerate() {
        let mut value = tokens[i];
        match payload.arg.r#type.as_str() {
            "wstr"
            | "str" => {
                if value.len() < 3 {
                    println!("Invalid value {} for field {}", value, payload.arg.name);
                    return;
                }

                if !value.starts_with('"') || !value.ends_with('"') {
                    println!("Invalid value {} for field {}", value, payload.arg.name);
                    return;
                }

                value = &value[1..];

                value = &value[..value.len() - 1];

                if let Ok(_) = value.parse::<i64>() {
                    println!("Can't apply number to {} for field {}", payload.arg.r#type, payload.arg.name);
                    return;
                }
            }
            "int" => {
                let result = value.parse::<i32>();

                if result.is_err() {
                    println!("Can't parse value {} to int 32 for field {}", payload.arg.r#type, payload.arg.name);
                    return;
                }
            }
            "short" => {
                let result = value.parse::<i16>();

                if result.is_err() {
                    println!("Can't parse value {} to short 16 for field {}", payload.arg.r#type, payload.arg.name);
                    return;
                }
            }
            _ => {}
        }


        payload.value = value.to_string();
    }

    if let Some(config_file) =
        config_files.unwrap().iter().find(|x| x.name.eq(state.current_name.as_ref().unwrap())) {
        let arch = if cfg!(target_pointer_width = "64") {
            "amd64"
        } else if cfg!(target_pointer_width = "32") {
            "i386"
        } else {
            return;
        };

        let os_file_path = if let Some(os_file) = config_file.files.iter().find(|x| x.arch.eq(arch)) {
            os_file.path.clone()
        } else {
            return;
        };

        let os_file_path = Path::new(CONFIGS_PATH)
            .join(Path::new(&config_file.name))
            .join(os_file_path);

        let result = file_service::read_file_to_u8_array(os_file_path.to_str().unwrap());

        if result.is_err() {
            println!("Can't read {:?}", os_file_path);
            return;
        }

        let bytes = result.unwrap();

        match Coffee::new(&bytes) {
            Ok(coffee) => {
                let arguments = hexlify_args(&state.payloads);

                if arguments.is_err() {
                    println!("Can't parse arguments: {}", arguments.err().unwrap());
                    return;
                }

                let arguments = arguments.unwrap();

                let unhexilified = unhexilify_args(arguments.as_str());

                if unhexilified.is_err() {
                    println!("Can't unhexilifid arguments: {}", unhexilified.err().unwrap());
                    return;
                }

                let unhexilified = unhexilified.unwrap();

                match coffee.execute(
                    Some(unhexilified.as_ptr()),
                    Some(unhexilified.len()),
                    None,
                ) {
                    Ok(res) => {
                        println!("{res}");
                    }
                    Err(e) => {
                        println!("Execution failed: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error creating Coffee object: {:?}", e);
            }
        }
    }
}

fn unhexilify_args(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("Invalid argument hexadecimal string".to_string());
    }

    let bytes: Result<Vec<u8>, _> = (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16))
        .collect();

    Ok(bytes.unwrap())
}

fn hexlify_args(payloads: &Vec<Payload>) -> Result<String, String> {
    let mut beacon_pack = BeaconPack::new();

    for payload in payloads {
        match payload.arg.r#type.as_str() {
            "str" => beacon_pack.add_str(&payload.value),
            "wstr" => beacon_pack.add_wstr(&payload.value),
            "int" => {
                if let Ok(int_value) = payload.value.parse::<i32>() {
                    beacon_pack.add_int(int_value);
                } else {
                    return Err("Invalid integer value".to_string());
                }
            }
            "short" => {
                if let Ok(short_value) = payload.value.parse::<i16>() {
                    beacon_pack.add_short(short_value);
                } else {
                    return Err("Invalid short value".to_string());
                }
            }
            _ => return Err("Invalid argument type".to_string()),
        }
    }

    let hex_buffer = beacon_pack
        .get_buffer()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect();

    Ok(hex_buffer)
}

fn print_options(state: &mut State) {
    if state.current_name.is_none() {
        println!("Now no config file selected");
        return;
    }

    let config_files = file_service::get_all_config_files(CONFIGS_PATH);

    if config_files.is_err() {
        println!("Can't get config files");
        return;
    }

    if let Some(config_file) =
        config_files.unwrap().iter().find(|x| x.name.eq(state.current_name.as_ref().unwrap())) {
        println!("Options of config file: {}", config_file.name);

        println!("Repo url: {}", config_file.repo_url);
        println!("Description: {}", config_file.desc);
        println!();

        let mut table = Table::new();
        table.set_header(vec!["os", "arch", "path"]);

        for file in config_file.files.iter() {
            table.add_row(vec![&file.os, &file.arch, file.path.to_str().unwrap()]);
        }

        println!("Available os files:");
        println!("{table}");
        println!();

        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic);

        table
            .set_header(vec!["name", "type", "optional", "description"]);


        for payload in state.payloads.iter() {
            table.add_row(
                vec![
                    &payload.arg.name,
                    &payload.arg.r#type,
                    &payload.arg.optional.to_string(),
                    &payload.arg.desc,
                ]);
        }
        println!("List of arguments:");

        if !state.payloads.is_empty() {
            println!("{table}");
        } else {
            println!("No arguments available")
        }
        println!();
    } else {
        println!("Can't find config file with name {}", state.current_name.as_ref().unwrap());
    }
}

fn print_current(state: &mut State) {
    match state.current_name.as_ref() {
        None => println!("Now no config file selected"),
        Some(name) => println!("Now selected config with name {}", name),
    };
}

fn print_help() {
    let mut table = Table::new();

    table
        .set_content_arrangement(ContentArrangement::Dynamic);

    table
        .set_header(vec!["Name", "Description", "Example"])
        .add_row(vec!["help", "Get all user commands", "help"])
        .add_row(vec!["list", "Print all configs", "list"])
        .add_row(vec!["clear", "Clear console", "clear"])
        .add_row(vec!["search [name | description]", "Search configs by name", "search enum"])
        .add_row(vec!["use [name | index]", "Select config by name or by index in current list or search", "use 2"])
        .add_row(vec!["cur", "Print selected config", "cur"])
        .add_row(vec!["options", "Print options for selected config file", "options"])
        .add_row(vec!["run [..args]", "Run current config file", "run \"C:\\\" 5005"])
        .add_row(vec!["exit", "Exit from app", "exit"]);

    println!("{table}");
}

fn handle_use_command(input: &str, state: &mut State) {
    let use_term = &input[4..];
    state.current_name = None;
    state.payloads = vec![];

    let config_files: Vec<ConfigFile> = if state.list.is_empty() {
        file_service::get_all_config_files(CONFIGS_PATH).unwrap_or_else(|_| vec![])
    } else {
        state.list.clone()
    };


    if let Ok(index) = use_term.parse::<usize>() {
        if let Some(config_file) = config_files.get(index) {
            state.current_name = Some(config_file.name.clone());

            for argument in config_file.arguments.iter() {
                state.payloads.push(Payload { value: String::new(), arg: argument.clone() })
            }
        }
    } else {
        let config_files = file_service::get_all_config_files(CONFIGS_PATH)
            .unwrap_or_else(|_| vec![]);

        if let Some(config_file) = config_files.iter().find(|x| x.name.eq(use_term)) {
            state.current_name = Some(config_file.name.clone());

            for argument in config_file.arguments.iter() {
                state.payloads.push(Payload { value: String::new(), arg: argument.clone() })
            }
        }
    }

    match state.current_name.as_ref() {
        None => println!("Can't select config with name or index {}", use_term),
        Some(name) => {
            println!("Selected config with name {}", name);

            if !state.payloads.is_empty() {
                let args_str = state.payloads
                    .iter()
                    .map(|x| {
                        let mut str = String::new();

                        if !x.arg.optional {
                            str = "*".to_string();
                        }

                        str = format!("{}{} {}", str, x.arg.name, x.arg.r#type);
                        return str.to_string();
                    }).collect::<Vec<String>>().join(", ");

                println!("help: {}", args_str);
            }
        }
    }
}

fn print_list(state: &mut State) {
    let config_files = file_service::get_all_config_files(CONFIGS_PATH);

    if config_files.is_err() {
        println!("Can't print list\n{:?}", config_files.err());
        return;
    }

    let config_files = config_files.unwrap();

    let mut table = Table::new();

    table
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec!["#", "Name", "Description"]);

    for (i, config_file) in config_files.iter().enumerate() {
        table.add_row(vec![&i.to_string(), &config_file.name, &config_file.desc]);
    }

    state.list = vec![];

    println!("{table}");
}

fn print_search(input: &str, state: &mut State) {
    let config_files = file_service::get_all_config_files(CONFIGS_PATH);

    if config_files.is_err() {
        println!("Can't get search list\n{:?}", config_files.err());
        return;
    }

    let search_term = &input[7..];

    let config_files = config_files.unwrap();

    let config_files: Vec<ConfigFile> = config_files
        .iter()
        .filter(|x| x.name.contains(search_term) || x.desc.contains(search_term))
        .map(|x| x.clone())
        .collect();

    let mut table = Table::new();

    table
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec!["#", "Name", "Description"]);

    for (i, config_file) in config_files.iter().enumerate() {
        table.add_row(vec![&i.to_string(), &config_file.name, &config_file.desc]);
    }

    state.list = config_files.clone();

    println!("{table}");
}

fn print_unknown_command(input: &str) {
    print!("Can't find command \"{}\"\n", input);
}

fn clean_screen() {
    clearscreen::clear().unwrap();
}

fn get_user_input() -> Result<String, io::Error> {
    let mut input = String::new();

    print!("> ");
    io::stdout().flush()?;

    io::stdin()
        .read_line(&mut input)?;

    Ok(input.trim().to_string())
}

