# スキルチェック課題 (2/2)
## 仕様
linuxのsysctl.confと同じ文法の任意のファイルをロードして、Json形式で標準出力する。

ファイル内容の不備をスキーマファイルを用いて確認。
スキーマファイルの例は下記の通り。
```
endpoint -> string
debug -> bool
log.file -> string
```

ファイル指定は実行引数として、第一引数にスキーマファイルを、それ以降にファイルパスまたはファイルが格納されているディレクトリパスを渡す。

なお、ファイル及びディレクトリは複数指定可能。
### 実行コマンド例
```
cargo run -- ./check.schema ./input_files/test1.txt ./input_files/test2.conf #複数ファイル指定
cargo run -- ./check.schema ./input_files/test_files #ディレクトリ指定
```
