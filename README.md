# OpenFX Network Video Plugin

DaVinci Resolve 向けの映像専用 OpenFX フィルタープラグイン

NDI® is a registered trademark of Vizrt NDI AB.

## 要件

- Windows x64
- DaVinci Resolve（OpenFX ホスト）

## インストール

[Releases](https://github.com/MikanseiLaboratory/openfx-network-video-plugin/releases) の `openfx-network-video-plugin-v*.zip` を展開し、`OpenFXNetworkVideo.ofx.bundle` を次の場所へコピーします。

```text
C:\Program Files\Common Files\OFX\Plugins
```

## 使い方

1. クリップに **Network Video Output** を適用する
2. **Enabled** をオンにする
3. **Source Name** と、必要なら **Groups** を変更する
4. 受信側で `パソコンのNDI名 (ソース名)` を選ぶ

## ライセンス

プラグイン本体: MIT  
サードパーティライブラリは `THIRD_PARTY_NOTICES.md`をご確認ください。  
OpenFX ヘッダーは Academy Software Foundation の BSD-3-Clause です。

同梱 NDI® runtime (`Processing.NDI.Lib.x64.dll`) には `NDI_TERMS.txt` と `Processing.NDI.Lib.Licenses.txt` が適用されます。
