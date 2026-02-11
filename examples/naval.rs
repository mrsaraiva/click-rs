//! Naval Fate - A demonstration of click-rs command groups.
//!
//! This is the docopt example adopted to Click but with some actual
//! commands implemented and not just the empty parsing which really
//! is not all that interesting.
//!
//! Equivalent to Python Click's examples/naval/naval.py
//!
//! Features demonstrated:
//! - Nested command groups (cli -> ship, cli -> mine)
//! - Arguments with different types (string, float)
//! - Options with defaults
//! - Flag options with flag_value

use std::env;

use click::{
    Argument, ClickError, ClickOption, Command, Context, Group, Result,
    group::CommandLike,
};

/// Build the main Naval Fate CLI.
fn build_cli() -> Group {
    Group::new("naval")
        .help("Naval Fate.\n\n\
               This is the docopt example adopted to Click but with some actual \
               commands implemented and not just the empty parsing which really \
               is not all that interesting.")
        .option(
            ClickOption::new(&["--version", "-V"])
                .flag("true")
                .eager()
                .help("Show version and exit.")
                .metavar("__click_version__:Naval Fate 2.0")
                .build(),
        )
        .command(build_ship_group())
        .command(build_mine_group())
        .build()
}

/// Build the `ship` command group.
fn build_ship_group() -> Group {
    Group::new("ship")
        .help("Manages ships.")
        .command(build_ship_new_command())
        .command(build_ship_move_command())
        .command(build_ship_shoot_command())
        .build()
}

/// Build the `ship new` command.
fn build_ship_new_command() -> Command {
    Command::new("new")
        .help("Creates a new ship.")
        .argument(
            Argument::new("name")
                .help("Name of the ship to create")
                .build(),
        )
        .callback(ship_new_callback)
        .build()
}

fn ship_new_callback(ctx: &Context) -> Result<()> {
    let name = ctx
        .get_param::<String>("name")
        .ok_or_else(|| ClickError::missing_argument("NAME"))?;

    println!("Created ship {}", name);
    Ok(())
}

/// Build the `ship move` command.
fn build_ship_move_command() -> Command {
    Command::new("move")
        .help("Moves SHIP to the new location X,Y.")
        .argument(
            Argument::new("ship")
                .help("Ship to move")
                .build(),
        )
        .argument(
            Argument::new("x")
                .help("X coordinate")
                .build(),
        )
        .argument(
            Argument::new("y")
                .help("Y coordinate")
                .build(),
        )
        .option(
            ClickOption::new(&["--speed", "-s"])
                .metavar("KN")
                .default("10")
                .help("Speed in knots.")
                .build(),
        )
        .callback(ship_move_callback)
        .build()
}

fn ship_move_callback(ctx: &Context) -> Result<()> {
    let ship = ctx
        .get_param::<String>("ship")
        .ok_or_else(|| ClickError::missing_argument("SHIP"))?;
    let x = ctx
        .get_param::<String>("x")
        .ok_or_else(|| ClickError::missing_argument("X"))?;
    let y = ctx
        .get_param::<String>("y")
        .ok_or_else(|| ClickError::missing_argument("Y"))?;
    let speed = ctx
        .get_param::<String>("speed")
        .map(|s| s.as_str())
        .unwrap_or("10");

    // Parse coordinates as floats
    let x_val: f64 = x
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", x)))?;
    let y_val: f64 = y
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", y)))?;

    println!("Moving ship {} to {},{} with speed {}", ship, x_val, y_val, speed);
    Ok(())
}

/// Build the `ship shoot` command.
fn build_ship_shoot_command() -> Command {
    Command::new("shoot")
        .help("Makes SHIP fire to X,Y.")
        .argument(
            Argument::new("ship")
                .help("Ship to fire from")
                .build(),
        )
        .argument(
            Argument::new("x")
                .help("X coordinate")
                .build(),
        )
        .argument(
            Argument::new("y")
                .help("Y coordinate")
                .build(),
        )
        .callback(ship_shoot_callback)
        .build()
}

fn ship_shoot_callback(ctx: &Context) -> Result<()> {
    let ship = ctx
        .get_param::<String>("ship")
        .ok_or_else(|| ClickError::missing_argument("SHIP"))?;
    let x = ctx
        .get_param::<String>("x")
        .ok_or_else(|| ClickError::missing_argument("X"))?;
    let y = ctx
        .get_param::<String>("y")
        .ok_or_else(|| ClickError::missing_argument("Y"))?;

    // Parse coordinates as floats
    let x_val: f64 = x
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", x)))?;
    let y_val: f64 = y
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", y)))?;

    println!("Ship {} fires to {},{}", ship, x_val, y_val);
    Ok(())
}

/// Build the `mine` command group.
fn build_mine_group() -> Group {
    Group::new("mine")
        .help("Manages mines.")
        .command(build_mine_set_command())
        .command(build_mine_remove_command())
        .build()
}

/// Build the `mine set` command.
fn build_mine_set_command() -> Command {
    Command::new("set")
        .help("Sets a mine at a specific coordinate.")
        .argument(
            Argument::new("x")
                .help("X coordinate")
                .build(),
        )
        .argument(
            Argument::new("y")
                .help("Y coordinate")
                .build(),
        )
        .option(
            ClickOption::new(&["--moored", "-m"])
                .dest("ty")
                .flag("moored")
                .default("moored")
                .help("Moored (anchored) mine. Default.")
                .build(),
        )
        .option(
            ClickOption::new(&["--drifting", "-d"])
                .dest("ty")
                .flag("drifting")
                .help("Drifting mine.")
                .build(),
        )
        .callback(mine_set_callback)
        .build()
}

fn mine_set_callback(ctx: &Context) -> Result<()> {
    let x = ctx
        .get_param::<String>("x")
        .ok_or_else(|| ClickError::missing_argument("X"))?;
    let y = ctx
        .get_param::<String>("y")
        .ok_or_else(|| ClickError::missing_argument("Y"))?;

    // Parse coordinates as floats
    let x_val: f64 = x
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", x)))?;
    let y_val: f64 = y
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", y)))?;

    let mine_type = ctx
        .get_param::<String>("ty")
        .map(|s| s.as_str())
        .unwrap_or("moored");

    println!("Set {} mine at {},{}", mine_type, x_val, y_val);
    Ok(())
}

/// Build the `mine remove` command.
fn build_mine_remove_command() -> Command {
    Command::new("remove")
        .help("Removes a mine at a specific coordinate.")
        .argument(
            Argument::new("x")
                .help("X coordinate")
                .build(),
        )
        .argument(
            Argument::new("y")
                .help("Y coordinate")
                .build(),
        )
        .callback(mine_remove_callback)
        .build()
}

fn mine_remove_callback(ctx: &Context) -> Result<()> {
    let x = ctx
        .get_param::<String>("x")
        .ok_or_else(|| ClickError::missing_argument("X"))?;
    let y = ctx
        .get_param::<String>("y")
        .ok_or_else(|| ClickError::missing_argument("Y"))?;

    // Parse coordinates as floats
    let x_val: f64 = x
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", x)))?;
    let y_val: f64 = y
        .parse()
        .map_err(|_| ClickError::usage(format!("'{}' is not a valid float.", y)))?;

    println!("Removed mine at {},{}", x_val, y_val);
    Ok(())
}

fn main() {
    let cli = build_cli();
    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(e) = cli.main(args) {
        eprintln!("{}", e.format_full());
        std::process::exit(e.exit_code());
    }
}
