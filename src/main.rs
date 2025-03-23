use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use regex::Regex;
use lazy_static::lazy_static;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfigValue {
    String(String),
    Map(HashMap<String, ConfigValue>),
    Bool(bool),
}

lazy_static! {
    static ref CONFIG_REGEX: Regex = Regex::new(r"^\s*([a-zA-Z0-9._-]+)\s*=\s*(.+?)\s*$").unwrap();
    static ref COMMENT_REGEX: Regex = Regex::new(r"^\s*#").unwrap();
}

fn parse_config_file(file_path: &Path) -> io::Result<HashMap<String, ConfigValue>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut config = HashMap::new();

    for line in reader.lines().flatten() {
        let trimmed_line = line.trim();
        if COMMENT_REGEX.is_match(trimmed_line) || trimmed_line.is_empty() {
            continue; // コメント行・空行をスキップ
        }

        if let Some(captures) = CONFIG_REGEX.captures(trimmed_line) {
            let key = captures[1].to_string();
            let raw_value = captures[2].trim().to_string();
            let value = if raw_value.eq_ignore_ascii_case("true") || raw_value.eq_ignore_ascii_case("false") {
                ConfigValue::Bool(raw_value.eq_ignore_ascii_case("true"))
            } else {
                ConfigValue::String(raw_value)
            };
            insert_config_value(&mut config, &key, value);
        }
    }

    Ok(config)
}

fn insert_config_value(config: &mut HashMap<String, ConfigValue>, key: &str, value: ConfigValue) {
    let keys: Vec<&str> = key.split('.').collect();
    let mut map = config;

    for sub_key in &keys[..keys.len() - 1] {
        map = map.entry(sub_key.to_string())
            .or_insert_with(|| ConfigValue::Map(HashMap::new()))
            .as_map_mut()
            .expect("型の不一致");
    }

    map.insert(keys.last().unwrap().to_string(), value);
}

fn collect_text_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if path.is_dir() {
        return Ok(fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|p| p.is_file())
            .collect());
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "パスが見つかりません"))
}

fn format_as_json(config: &HashMap<String, ConfigValue>) -> serde_json::Value {
    let mut json_obj = serde_json::Map::new();
    for (key, value) in config {
        match value {
            ConfigValue::String(s) => {
                json_obj.insert(key.clone(), json!(s));
            }
            ConfigValue::Map(m) => {
                json_obj.insert(key.clone(), format_as_json(m));
            }
            ConfigValue::Bool(b) => {
                json_obj.insert(key.clone(), json!(b));
            }
        }
    }
    serde_json::Value::Object(json_obj)
}

fn validate_config(config: &HashMap<String, ConfigValue>, schema: &HashMap<String, String>) -> bool {
    let mut valid = true;
    for (key, expected_type) in schema {
        if let Some(value) = config.get(key) {
            match (expected_type.as_str(), value) {
                ("string", ConfigValue::String(_)) => (),
                ("bool", ConfigValue::Bool(_)) => (),
                ("map", ConfigValue::Map(_)) => (),
                _ => {
                    eprintln!("キー '{}' の値が期待される型 '{}' と一致しません", key, expected_type);
                    valid = false;
                }
            }
        } else {
            println!("警告: キー '{}' が存在しません", key);
        }
    }
    valid
}

fn load_schema(file_path: &Path) -> io::Result<HashMap<String, String>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut schema = HashMap::new();

    for line in reader.lines().flatten() {
        let trimmed_line = line.trim();
        if COMMENT_REGEX.is_match(trimmed_line) || trimmed_line.is_empty() {
            continue; // コメント行・空行をスキップ
        }

        if let Some(captures) = CONFIG_REGEX.captures(trimmed_line) {
            let key = captures[1].to_string();
            let value_type = captures[2].trim().to_string();
            schema.insert(key, value_type);
        }
    }

    Ok(schema)
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
                    Err(e) => eprintln!("エラー: ファイルの読み込みに失敗しました: {} ({})", e, file.display()),
                }
            }),
            Err(e) => eprintln!("エラー: ファイルの収集に失敗しました: {} ({})", e, config_path.display()),
        },
        Err(e) => eprintln!("エラー: スキーマファイルの読み込みに失敗しました: {} ({})", e, schema_path.display()),
    };
}

impl ConfigValue {
    fn as_map_mut(&mut self) -> Option<&mut HashMap<String, ConfigValue>> {
        if let ConfigValue::Map(m) = self {
            Some(m)
        } else {
            None
        }
    }
}
