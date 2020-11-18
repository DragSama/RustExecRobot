extern crate serde_json;

use carapax::{
    handler,
    Api,
    Dispatcher,
    ExecuteError,
    longpoll::LongPoll,
    methods::{SendMessage},
    types::{
        Command
    }
};

use reqwest::Client;
use serde::Deserialize;
use std::{
    env,
    convert::Infallible
};

struct Context {
    api: Api,
    users: Vec<i64>,
    reqwest_client: Client
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="PascalCase")]
struct Data {
    warnings: Option<String>,
    errors: Option<String>,
    result: Option<String>,
    stats: String
}

async fn is_valid(_context: &Context, command: &Command) -> Result<bool, Infallible> {
    Ok(command.get_message().get_text().unwrap().data.starts_with("/rustexec"))
}


#[handler(predicate=is_valid)]
async fn exec_handler(context: &Context, command: Command) -> Result<(), ExecuteError> {
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
    let data = vec![("LanguageChoice", "46"), ("Program", &code)];
    let resp = match context.reqwest_client.post("https://rextester.com/rundotnet/api")
            .form(&data)
            .send()
            .await{
                Ok(resp) => match resp.text().await {
                    Ok(resp) => resp,
                    Err(err) => {
                        eprintln!("ERROR: {}", err);
                        return Ok(());
                    }
                },
                Err(err) => {
                    eprintln!("ERROR: {}", err);
                    return Ok(());
                }
            };
    let output: Data = match serde_json::from_str(&resp) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("ERROR: {}", err);
            return Ok(());
        }
    };
    if !output.result.is_some(){
        let mut reply_message = String::from("Error:\n");
        reply_message.push_str(output.errors.unwrap().as_str());
        if output.warnings.is_some(){
            reply_message.push_str("\nWarnings:\n");
            reply_message.push_str(output.warnings.unwrap().as_str());
        };
        let method = SendMessage::new(chat_id, reply_message);
        context.api.execute(method).await?;
    } else {
        let mut reply_message = String::from("Output:\n");
        reply_message.push_str(output.result.unwrap().as_str());
        if output.warnings.is_some(){
            reply_message.push_str("\nWarnings:\n");
            reply_message.push_str(output.warnings.unwrap().as_str());
        };
        let method = SendMessage::new(chat_id, reply_message);
        context.api.execute(method).await?;
    };
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = env::var("TOKEN").expect("TOKEN is not set");
    let allowed = env::var("RUSERS").expect("RUSERS are not set.");
    let split = allowed.split(" ");
    let users: Vec<i64> = split.into_iter().map(|x| x.parse::<i64>().unwrap()).collect();
    let api = Api::new(token).expect("Failed to create API");
    let client = Client::new();

    let mut dispatcher = Dispatcher::new(
        Context {api: api.clone(), users: users, reqwest_client: client}
    );

    dispatcher.add_handler(exec_handler);

    LongPoll::new(api, dispatcher).run().await
}
