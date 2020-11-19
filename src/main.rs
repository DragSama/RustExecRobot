use carapax::{
    handler,
    Api,
    Dispatcher,
    ExecuteError,
    longpoll::LongPoll,
    methods::{SendMessage},
    types::{
        Command as BotCommand,
        ParseMode
    }
};

use std::{
    env,
    fs,
    path::Path,
    convert::Infallible,
    process::Command,
    io::Write
};

struct Context {
    api: Api,
    users: Vec<i64>
}


async fn is_valid(_context: &Context, command: &BotCommand) -> Result<bool, Infallible> {
    Ok(command.get_message().get_text().unwrap().data.starts_with("/rustexec"))
}

async fn shell_is_valid(_context: &Context, command: &BotCommand) -> Result<bool, Infallible> {
    Ok(command.get_message().get_text().unwrap().data.starts_with("/sh"))
}

#[handler(predicate=shell_is_valid)]
async fn shell_handler(context: &Context, command: BotCommand) -> Result<(), ExecuteError> {
    let user_id = command.get_message().get_user().unwrap().id;
    let chat_id = command.get_message().get_chat_id();
    let args = command.get_args();
    if !context.users.contains(&user_id){
        return Ok(());
    };

    if args.len() == 0 {
        let method = SendMessage::new(chat_id, "Format: /sh code");
        context.api.execute(method).await?;
        return Ok(());
    }

    // let code: String = args.into_iter().map(|i| i.to_string()).collect();
    // turns out args doesn't preserve space

    let cmd = command.get_message()
        .get_text()
        .unwrap()
        .data.as_str()
        .replace("/sh", "");
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(cmd).output().expect("Error occured while running cargo run")
    } else {
        Command::new("sh").arg("-c").arg(cmd).output().expect("Error occured while running cargo run")
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let msg = format!("*STDERR:*\n`{}`\n*STDOUT*:\n`{}`", stderr, stdout);
    let method = SendMessage::new(chat_id, msg)
                .parse_mode(ParseMode::Markdown);
    context.api.execute(method).await?;
    Ok(())
}

#[handler(predicate=is_valid)]
async fn exec_handler(context: &Context, command: BotCommand) -> Result<(), ExecuteError> {
    let user_id = command.get_message().get_user().unwrap().id;
    let chat_id = command.get_message().get_chat_id();
    let args = command.get_args();
    if !context.users.contains(&user_id){
        return Ok(());
    };

    if args.len() == 0 {
        let method = SendMessage::new(chat_id, "Format: /rustexec code");
        context.api.execute(method).await?;
        return Ok(());
    }

    // let code: String = args.into_iter().map(|i| i.to_string()).collect();
    // turns out args doesn't preserve space

    let code = command.get_message()
        .get_text()
        .unwrap()
        .data.as_str()
        .replace("/rustexec", "");
    let path = Path::new("builds/bot/src/main.rs");

    let mut file = match fs::File::create(&path) {
        Err(err) => {
            eprintln!("ERROR: {}", err);
            return Ok(())
        },
        Ok(file) => file
    };
    match file.write_all(code.as_bytes()){
        Err(err) => {
            eprintln!("ERROR: {}", err);
            return Ok(())
        },
        Ok(_) => println!("Running code: {}", code)
    };
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg("cd builds && cd bot && cargo run").output().expect("Error occured while running cargo run")
    } else {
        Command::new("sh").arg("-c").arg("cd builds && cd bot && cargo run").output().expect("Error occured while running cargo run")
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let msg = format!("*STDERR:*\n`{}`\n*STDOUT*:\n`{}`", stderr, stdout);
    let method = SendMessage::new(chat_id, msg)
                .parse_mode(ParseMode::Markdown);
    context.api.execute(method).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = env::var("TOKEN").expect("TOKEN is not set");
    let allowed = env::var("RUSERS").expect("RUSERS are not set.");
    let split = allowed.split(" ");
    let users: Vec<i64> = split.into_iter().map(|x| x.parse::<i64>().unwrap()).collect();
    let api = Api::new(token).expect("Failed to create API");

    let mut dispatcher = Dispatcher::new(
        Context {api: api.clone(), users: users}
    );
    if !Path::new("builds").is_dir(){
        println!("builds folder does not exist, Creating...");
        fs::create_dir("builds").expect("Failed to create builds folder.");
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                    .args(&["/C", "cargo new builds/bot"])
                    .output()
                    .expect("failed to run cargo new")
        } else {
            Command::new("sh")
                    .arg("-c")
                    .arg("cargo new builds/bot")
                    .output()
                    .expect("failed to run cargo new")
        };
        println!("Created builds folder.");
    }
    dispatcher.add_handler(shell_handler);
    dispatcher.add_handler(exec_handler);
    LongPoll::new(api, dispatcher).run().await
}
