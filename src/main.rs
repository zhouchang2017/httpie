use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use mime::Mime;
use reqwest::{header, Client, Response, Url};
use syntect::{parsing::SyntaxSet, highlighting::{ThemeSet, Style}, easy::HighlightLines, util::{LinesWithEndings, as_24_bit_terminal_escaped}};

/// A naive httpie implementation with Rust, can you imagine how easy it is?
#[derive(Debug, Parser)]
#[clap(
    version = "1.0",
    author = "github.com/zhouchang2017<zhouchangqaz@gmail.com>"
)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

/// <GET|POST>
#[derive(Debug, Parser)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

// get 子命令

/// feed get with an url and we will retrieve the response for you
#[derive(Debug, Parser)]
struct Get {
    /// HTTP 请求的 URL
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}

fn parse_url(s: &str) -> Result<String> {
    let _: Url = s.parse()?;
    Ok(s.into())
}

// post 子命令。需要输入一个 url，和若干个可选的 key=value，用于提供 json body

/// feed post with an url and optional key=value pairs. We will post the data as JSON, and retrieve the response for you
#[derive(Debug, Parser)]
struct Post {
    /// HTTP 请求的 URL
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    /// HTTP 请求的 body
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}

/// 命令行中的 key=value 可以通过 parse_kv_pair 解析成 KvPair 结构
#[derive(Debug, PartialEq)]
struct KvPair {
    k: String,
    v: String,
}

impl FromStr for KvPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("=");
        let err = || anyhow!(format!("Failed to parse {}", s));
        Ok(Self {
            k: (split.next().ok_or_else(err)?).to_string(),
            v: (split.next().ok_or_else(err)?).to_string(),
        })
    }
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    Ok(s.parse()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    let client = Client::new();
    let result = match opts.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?,
    };

    Ok(result)
}

async fn get(client: Client, args: &Get) -> Result<()> {
    let resp = client.get(&args.url).send().await?;
    Ok(print_resp(resp).await?)
}

async fn post(client: Client, args: &Post) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.k, &pair.v);
    }
    let resp = client.post(&args.url).json(&body).send().await?;
    Ok(print_resp(resp).await?)
}

async fn print_resp(resp: Response) -> Result<()> {
    print_status(&resp);
    print_headers(&resp);
    let mime = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(mime, &body);
    Ok(())
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }
    println!("");
}

fn print_body(m: Option<Mime>, body: &String) {
    match m {
        Some(v) if v == mime::APPLICATION_JSON=> print_syntect(body, "json"),
        Some(v) if v == mime::TEXT_HTML=> print_syntect(body, "html"),
        _ => println!("{}", body),
    }
}

fn print_syntect(s:&str,ext:&str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in LinesWithEndings::from(s) { // LinesWithEndings enables use of newlines mode
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..],true);
        println!("{}", escaped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_it_works() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("http://abc.com").is_ok());
        assert!(parse_url("http://api.test.com/v1?a=3").is_ok())
    }

    #[test]
    fn parse_kv_pair_it_works() {
        assert!(parse_kv_pair("a").is_err());
        assert_eq!(
            parse_kv_pair("a=1").unwrap(),
            KvPair {
                k: "a".into(),
                v: "1".into()
            }
        );
        assert_eq!(
            parse_kv_pair("a=").unwrap(),
            KvPair {
                k: "a".into(),
                v: "".into(),
            }
        )
    }
}
