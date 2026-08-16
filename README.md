# OpenFX Network Video Plugin

DaVinci Resolve 向けの映像専用 OpenFX フィルターです。タイムラインの絵を変えずに、有効時だけ [NDI®](https://ndi.video/) で送出します。

NDI® is a registered trademark of Vizrt NDI AB.

## 要件

- Windows x64
- DaVinci Resolve（OpenFX ホスト）
- 映像のみ（OpenFX Image Effect では音声を扱えません）

リリース package に `Processing.NDI.Lib.x64.dll` を同梱します。

## インストール

[Releases](https://github.com/MikanseiLaboratory/openfx-network-video-plugin/releases) の `openfx-network-video-plugin-v*.zip` を展開し、`OpenFXNetworkVideo.ofx.bundle` を次の場所へコピーします。

```text
C:\Program Files\Common Files\OFX\Plugins
```

Resolve を起動し直すと、OpenFX の **Mikansei Laboratory / Network Video Output** から使えます。

## 使い方

1. クリップに **Network Video Output** を適用する
2. **Enabled** をオンにする
3. **Source Name** と、必要なら **Groups** を変更する
4. 受信側で `パソコンのNDI名 (ソース名)` を選ぶ

映像は常に入力から出力へそのままコピーされます。Enabled がオフのときは送出しません。先頭の名前は PC 側の NDI® 名で、プラグインから変更できません。

## 削除

Resolve を終了してから `C:\Program Files\Common Files\OFX\Plugins\OpenFXNetworkVideo.ofx.bundle` を削除します。

## 開発

ビルドには Rust 1.97、LLVM（bindgen / libclang）、NDI 6 SDK が必要です。

```powershell
# NDI 6 SDK: https://ndi.video/type/developer/
# 既定パス: C:\Program Files\NDI\NDI 6 SDK
# または NDI_SDK_DIR を設定します。
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --test-threads=1
cargo build --release --locked --target x86_64-pc-windows-msvc
./scripts/package.ps1
```

Resolve での確認手順は `docs/resolve-verification.md` です。

## ライセンス

プラグイン本体は MIT。第三者通知は `THIRD_PARTY_NOTICES.md`。OpenFX ヘッダーは Academy Software Foundation の BSD-3-Clause です。

同梱 NDI® runtime (`Processing.NDI.Lib.x64.dll`) には `NDI_TERMS.txt` と `Processing.NDI.Lib.Licenses.txt` が適用されます。
