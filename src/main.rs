use std::{
    collections::HashMap,
    env, fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfigValue {
    String(String),
    Bool(bool),
    Map(HashMap<String, ConfigValue>),
}

lazy_static! {
    static ref CONFIG_REGEX: Regex = Regex::new(r"^\s*([a-zA-Z0-9._-]+)\s*=\s*(.+?)\s*$").unwrap();
    static ref COMMENT_REGEX: Regex = Regex::new(r"^\s*#").unwrap();
}

fn parse_config_file(file_path: &Path) -> io::Result<HashMap<String, ConfigValue>> {
    let reader = BufReader::new(fs::File::open(file_path)?);

    reader
        .lines()
        .flatten()
        .filter(|line| !COMMENT_REGEX.is_match(line))
        .filter_map(|line| {
            CONFIG_REGEX.captures(&line).map(|caps| {
                let key = caps[1].to_string();
                let value = match caps[2].trim().to_lowercase().as_str() {
                    "true" => ConfigValue::Bool(true),
                    "false" => ConfigValue::Bool(false),
                    raw => ConfigValue::String(raw.to_string()),
                };
                (key, value)
            })
        })
        .fold(Ok(HashMap::new()), |acc, (key, value)| {
            acc.and_then(|mut map| {
                insert_config_value(&mut map, &key, value);
                Ok(map)
            })
        })
}

fn insert_config_value(config: &mut HashMap<String, ConfigValue>, key: &str, value: ConfigValue) {
    let keys: Vec<&str> = key.split('.').collect();
    keys.iter()
        .take(keys.len() - 1)
        .fold(config, |map, sub_key| {
            map.entry(sub_key.to_string())
                .or_insert_with(|| ConfigValue::Map(HashMap::new()))
                .as_map_mut()
                .expect("型の不一致: サブキーがマップではありません")
        })
        .insert(keys.last().unwrap().to_string(), value);
}

fn collect_text_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        Ok(vec![path.to_path_buf()])
    } else {
        Ok(fs::read_dir(path)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect::<Vec<_>>())
    }
}

fn format_as_json(config: &HashMap<String, ConfigValue>) -> Value {
    config
        .iter()
        .map(|(k, v)| {
            let value = match v {
                ConfigValue::String(s) => json!(s),
                ConfigValue::Bool(b) => json!(b),
                ConfigValue::Map(m) => format_as_json(m),
            };
            (k.clone(), value)
        })
        .collect::<Map<String, Value>>()
        .into()
}

fn load_schema(schema_path: &Path) -> io::Result<HashMap<String, String>> {
    BufReader::new(fs::File::open(schema_path)?)
        .lines()
        .flatten()
        .map(|line| {
            let parts: Vec<&str> = line.split("->").map(|s| s.trim()).collect();
            match parts.as_slice() {
                [key, "string"] | [key, "bool"] => Ok((key.to_string(), parts[1].to_string())),
                _ => Err(io::Error::new(io::ErrorKind::InvalidData, format!("無効なスキーマ行: {}", line))),
            }
        })
        .collect()
}

fn validate_config(config: &HashMap<String, ConfigValue>, schema: &HashMap<String, String>) -> bool {
    schema.iter().all(|(key, expected)| {
        match (config.get(key), expected.as_str()) {
            (Some(ConfigValue::String(_)), "string") | (Some(ConfigValue::Bool(_)), "bool") => true,
            (Some(ConfigValue::String(s)), "bool") if s.parse::<bool>().is_ok() => true,
            (None, _) => {
                eprintln!("警告: キー '{}' が不足しています。", key);
                true
            },
            _ => {
                eprintln!("エラー: キー '{}' の型が一致しません。", key);
                false
            },
        }
    })
}

impl ConfigValue {
    fn as_map_mut(&mut self) -> Option<&mut HashMap<String, ConfigValue>> {
        if let ConfigValue::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("使用方法: {} <スキーマファイル> <設定ファイルまたはディレクトリ>", args[0]);
        std::process::exit(1);
    }

    let schema_path = Path::new(&args[1]);
    let config_path = Path::new(&args[2]);

    match load_schema(schema_path) {
        Ok(schema) => match collect_text_files(config_path) {
            Ok(files) => files.iter().for_each(|file| {
                println!("=== ファイル: {} ===", file.display());
                match parse_config_file(file) {
                    Ok(config) => {
                        if validate_config(&config, &schema) {
                            println!("{}", serde_json::to_string_pretty(&format_as_json(&config)).unwrap());
                        } else {
                            eprintln!("エラー: 設定ファイルの検証に失敗しました: {}", file.display());
                        }
                    }
                    Err(_) => eprintln!("エラー: ファイルの読み込みに失敗しました: {}", file.display()),
                }
            }),
            Err(_) => eprintln!("エラー: ファイルの収集に失敗しました: {}", config_path.display()),
        },
        Err(_) => eprintln!("エラー: スキーマファイルの読み込みに失敗しました: {}", schema_path.display()),
    };
}
