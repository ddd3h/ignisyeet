.. _page-dev-build-release:

ビルドとリリース
==================

ソースからのビルド、テスト、ドキュメントの作成、CI、リリースの手順を述べる。

.. _sec-dev-build:

ソースのビルドとテスト
------------------------

必要なものは、Rust（stable、最低 1.87）と ``cargo`` である。作図には Python 3.10 以上と、``uv`` または ``venv`` と ``pip`` を使う。

.. code-block:: sh

   cargo build --release
   cargo test --release
   cargo clippy --all-targets -- -D warnings
   cargo fmt --all -- --check
   cargo doc --no-deps --document-private-items   # 実装メモ（rustdoc）を読む
   bash scripts/test-install-codes.sh              # install.sh のエラーコード

作図の単体テストは ``uv run --project python python -m unittest discover -s python`` で実行する。
書式は ``rustfmt.toml``\ （``max_width = 160``）、Rust のツールチェーンは ``rust-toolchain.toml`` で定める。

テストには、設定キーとドキュメントの対応、エラーコードとドキュメントの対応、サンプルの結果（頂点高度、着地点）の固定が含まれる。
SU2、gmsh、MPI を使うテストは既定では実行せず、ツールを用意して ``cargo test --release -p cfd -- --ignored`` で実行する。

.. _sec-doc-build:

ドキュメントのビルド
----------------------

このドキュメント（Sphinx）は ``doc/`` にある。図は TikZ（LuaLaTeX + dvisvgm）と matplotlib で作る。
matplotlib の図に使うデータは本体を実行して作る（``make data``。最初に ``cargo build --release`` も走る）。
``make html`` と ``make pdf`` はどちらも、これらの図とデータを作り直してから文書を生成する。

.. code-block:: sh

   cd doc
   make html             # HTML を生成する。build/html/index.html を開く
   make pdf              # PDF を生成する。doc/build/latex/ignisyeet.pdf ができる

``make html`` は警告をエラーとして扱う（``-W``）。Python の環境は uv でも pip でも用意できる。
Makefile は既定では ``uv run --project .`` 経由で ``sphinx-build`` と ``python`` を呼ぶ（初回に依存パッケージが自動で入る）。

.. code-block:: sh

   # pip の場合: 仮想環境に入れて、PY を空にすると有効な環境の sphinx-build / python をそのまま呼ぶ
   python3 -m venv .venv && . .venv/bin/activate
   pip install -r doc/requirements.txt
   make html PY=
   make pdf  PY=

必要なソフトウェアは次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - ソフトウェア
     - 用途
   * - Rust（``cargo``）
     - 図のデータを作る本体のビルドと実行
   * - LuaLaTeX（luatexja）と dvisvgm
     - TikZ 図（HTML 用の SVG と PDF 用の PDF）
   * - inkscape
     - ロゴ（SVG）を PDF に変換する（PDF 版のみ）
   * - upLaTeX、dvipdfmx、latexmk
     - PDF 版の組版。TeX Live なら ``texlive-lang-japanese`` など日本語関連のパッケージを含めて入れる
   * - IPAex フォント
     - 図と PDF 本文の日本語

PDF の組版中に LaTeX がエラーで止まったときは、``doc/build/latex/ignisyeet.log`` を開き、``!`` で始まる行とその直後の行番号を確認する。
多くはパッケージ不足かフォント不足である。原因を直したあと、もう一度 ``make pdf`` を実行すればよい。

古いページ名の URL は、``conf.py`` の ``redirects`` で新しいページに転送する（アンカーは引き継がれない）。

.. _sec-dev-doc-scripts:

図とデータのスクリプト
------------------------

``doc/scripts/`` のスクリプトは、ドキュメントの図とデータを作る。保存済みのデータは ``doc/data/`` にある。

.. list-table:: ``doc/scripts/`` のスクリプト
   :header-rows: 1
   :widths: 34 66

   * - スクリプト
     - 内容
   * - ``build_tikz.py``
     - ``doc/figures/tikz/*.tex`` を SVG と PDF にする
   * - ``make_plots.py``
     - matplotlib の図を作る（本体の出力と ``doc/data/`` を読む）
   * - ``make_screens.py``
     - 端末の出力を疑似端末で受け、画面を PNG にする
   * - ``make_ogp.py``
     - リンクの共有用画像を作る
   * - ``extract_cfd_fields.py``、``cfd_axial_split.py``
     - 終わった CFD の実行から、図に使う流れの場とフィンの面ごとの軸力を取り出す
   * - ``basic_finner_cfd.py``、``basic_finner_compare.py``
     - Basic Finner の CFD と係数表を、実測と比べる
   * - ``rans_*.py``
     - RANS の検証の実行と集計（平板、円錐、サンプル機体、メッシュ、粗さ補正）
   * - ``perf_profile.sh``
     - 各段階の実行時間とプロファイルを測る（結果は ``doc/data/perf/``）

.. _sec-development:

CI とリリース
--------------

リリースとバージョン
~~~~~~~~~~~~~~~~~~~~

IgnisYeet のバージョンは ``Cargo.toml`` の ``[workspace.package] version``\ （``ignisyeet --version`` で表示される値）で決まり、
GitHub では同じ番号のタグ ``vX.Y.Z`` と GitHub Release で管理する。インストーラの user は、
``--version`` を指定しなければ最新のリリースを入れる。

CI
~~~

push とプルリクエストでは、ワークフロー ``ci.yml`` が ``cargo build --release --locked`` と ``cargo test --release --locked`` を実行する。
続けて、例題で ``geom``、``aero``、``sim``、風の格子の ``dispersion``、モンテカルロの ``dispersion``、``python/plot.py`` を通しで動かし、
``python/plot.py`` の単体テスト（``python/test_plot.py``）を実行し、``sim.max_time`` を短くした飛翔が終了ステータス 3 で終わることを確かめる。
clippy は ``-D warnings``、rustfmt は ``--check`` で実行し、違反があるとジョブが失敗する。
ShellCheck（``install.sh`` と ``scripts/*.sh``）も、指摘があるとジョブが失敗する。誤検出は、理由を書いた ``# shellcheck disable=SCxxxx`` の行で個別に除く。
別のジョブが、必要な最低の Rust の版 1.87 で ``cargo build --locked`` を実行する。

ワークフロー ``docs.yml`` は、プルリクエストと ``main`` への push で HTML のドキュメントを警告をエラーとして（``-W``）ビルドする。
GitHub Pages への公開は ``main`` への push（と手動の実行）のときだけで、プルリクエストではビルドだけを行う。
ワークフローが使う Action は、コミットの SHA で固定している。

リリース
~~~~~~~~

タグ ``v*`` を push すると、ワークフロー ``release.yml`` が次の処理を行う。

#. タグと ``Cargo.toml`` のバージョンが一致することを確かめる（一致しなければ失敗する）。
#. 4 つのターゲット向けにバイナリを作る。Linux は musl で静的にリンクするので、ディストリビューションによらず動く。

   .. list-table::
      :header-rows: 1
      :widths: 40 30 30

      * - ターゲット
        - ビルド環境
        - テスト
      * - ``x86_64-unknown-linux-musl``
        - Ubuntu（cargo-zigbuild）
        - 実行する
      * - ``aarch64-unknown-linux-musl``
        - Ubuntu（cargo-zigbuild）
        - 実行しない（クロスビルド）
      * - ``x86_64-apple-darwin``
        - macOS（Intel）
        - 実行する
      * - ``aarch64-apple-darwin``
        - macOS（Apple Silicon）
        - 実行する

#. ``scripts/package-release.sh`` で ``ignisyeet-<バージョン>-<ターゲット>.tar.gz`` と SHA-256 を作る。
   中身はバイナリ（``bin/ignisyeet``）、例題（Git で管理しているファイルだけ）、作図スクリプト（``python/plot.py`` と、依存関係の一覧および固定した版 ``uv.lock`` ``plot-constraints.txt``）、
   ``LICENSE``、``NOTICE``、``CITATION.cff``、``README.md`` である。
   アーカイブは GNU tar で、ファイルの順序、時刻（``SOURCE_DATE_EPOCH``、既定は 0）、所有者を固定して作る。macOS のビルドでは ``gnu-tar`` を入れる。
#. GitHub Release を作り、すべての配布物、チェックサム、``install.sh`` を添付する。
   リリースノートは前のタグからのコミットを Conventional Commits の種類（feat、fix、docs など）ごとにまとめて作る。

新しいバージョンを出すときは、``Cargo.toml``\ （と ``CITATION.cff`` などの表記）のバージョンを上げてコミットし、
同じ番号のタグを付けて push する。

.. code-block:: sh

   git tag v0.4.0
   git push origin v0.4.0
