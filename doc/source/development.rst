.. _page-dev:

開発者向け
==========

IgnisYeet のソースの構成、ビルドとテスト、ドキュメントの作成、リリースを述べる。
利用だけなら、この章は読まなくてよい。

.. _sec-dev-notes:

実装メモの置き場
----------------

利用者向けの章には、モデルと観測できる挙動だけを書く。計算の手順、ファイルの扱い、内部の定数といった実装の説明は、ソースコードのコメントに置く。
コメントは ``cargo doc --no-deps --document-private-items`` で読める（Python は docstring）。
:numref:`tbl-dev-notes` は、話題ごとの置き場である。

.. _tbl-dev-notes:

.. csv-table:: 実装メモの置き場
   :delim: ¦
   :header-rows: 1
   :widths: 40 60

   話題¦置き場
   設定の読み込み、設定の同一性の判定、係数表の再利用の条件¦``crates/cli/src/config.rs``、``crates/aero/src/table.rs``
   出力ファイルの書き出し¦``crates/cli/src/outfile.rs``
   KML の出力¦``crates/cli/src/kml.rs``
   作図（``ignisyeet-plot``）¦``python/plot.py`` の docstring
   STL の読み込みと形状の抽出¦``crates/geom/src/stl.rs``、``crates/geom/src/extract.rs``
   部品積み上げ法の係数表¦``crates/aero/src/model.rs``、``crates/aero/src/table.rs``
   パネル法のメッシュ、影響行列、解法¦``crates/panel/src/mesh.rs``、``influence.rs``、``solver.rs``、``supersonic.rs``
   CFD の SU2 設定ファイル¦``crates/cfd/src/su2cfg.rs``、``crates/cfd/src/su2_writer.py``
   CFD の壁面（STL、STEP、対称性、忠実度）¦``crates/cfd/src/surface.rs``、``stl_mesh.rs``、``step_mesh.rs``、``symmetry.rs``、``fidelity.rs``、``mesh_stl.py``、``mesh_step.py``、``stl_union.py``
   CFD の体積メッシュと境界層¦``crates/cfd/src/mesh.rs``、``mesh_gmsh.py``、``bl_layer.py``
   CFD のケースの実行、収束の判定、再開¦``crates/cfd/src/runner.rs``、``history.rs``、``case.rs``
   CFD の力の積分と係数表の作成¦``crates/cfd/src/forces.rs``、``table.rs``、``general.rs``、``asymmetry.rs``
   運動方程式と数値積分、ステップ内の事象¦``crates/sim/src/flight.rs``、``stepper.rs``、``integrator.rs``
   落下分散の乱数とサンプルの割り当て¦``crates/sim/src/dispersion.rs``

エラーコードとドキュメントの対応は :ref:`sec-dev-error-codes` にある。

.. _page-dev-architecture:

構成
----

.. _sec-dev-crates:

クレート構成
~~~~~~~~~~~~

計算は Rust のワークスペース（``crates/``）の 6 つのクレートで行い、作図は Python（``python/plot.py``）が受け持つ。
:numref:`fig-arch-crates` は、クレートの依存とデータの流れである。

.. _fig-arch-crates:

.. figure:: /_generated/tikz/arch_crates.*
   :width: 80%

   クレートの依存（実線の矢印は依存する向き）と、出力ファイルから作図へのデータの流れ（破線）。

.. csv-table:: クレートの役割
   :delim: ¦
   :header-rows: 1
   :widths: 14 86

   クレート¦役割
   ``geom``¦STL の入出力、機軸の決定、断面の切り出し、フィンの抽出、サンプル形状
   ``aero``¦標準大気、部品積み上げ法の空力モデル、係数表（保存・内挿・外挿）
   ``panel``¦パネル法の空力解析（亜音速の Morino 法、超音速の局所傾斜法）
   ``cfd``¦CFD による空力解析（gmsh のメッシュ、SU2 のケースの実行、係数表の作成）
   ``sim``¦推力曲線、6 自由度の飛翔と積分法、風・大気・重力、座標変換、落下分散
   ``cli``¦``ignisyeet`` コマンド、設定ファイル、出力ファイル、端末の表示

.. _sec-dev-flow:

データの流れ
~~~~~~~~~~~~

CFD で係数表を作る処理の流れを :numref:`fig-cfd-pipeline` に示す。
表面を STL から作る場合は、修復した STL を和集合にしたあとで壁面を作り直す。
基準の回転体から作る場合は、半径分布とフィン寸法から立体を作り直す。
どちらも体積メッシュと SU2 のケースに進み、解いた結果を機体軸の係数に直して係数表にする。

.. _fig-cfd-pipeline:

.. figure:: /_generated/tikz/cfd_pipeline.*
   :width: 100%

   CFD の係数表を作る処理。青は Rust、橙は Python（gmsh、OpenCASCADE、Manifold）、灰は入出力ファイルである。

.. _sec-dev-tree:

ディレクトリ構成
~~~~~~~~~~~~~~~~

リポジトリの主なファイルとディレクトリを :numref:`fig-directory-tree` に示す。

.. _fig-directory-tree:

.. figure:: /_generated/tikz/directory_tree.*
   :width: 100%

   リポジトリの構成。フォルダのアイコンはディレクトリ、紙のアイコンはファイルを表す。

.. _sec-dev-error-codes:

エラーコードとドキュメントの対応
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

エラーとワーニングのコード ``IY-<領域><番号>`` は、コード 1 つにつき 1 つのページ（``doc/source/errors/<領域>.rst`` の節）を持つ。
領域の文字とファイルの対応は ``C``\ （config）、``G``\ （geometry）、``A``\ （aero）、``S``\ （sim）、``O``\ （cli）、``F``\ （cfd）、``I``\ （install）である。
番号は 001〜499 がエラー、500〜999 がワーニングである。

次の対応はテストが確かめる。ソースに現れるコードはすべて文書にあり、文書にあるコードはすべてソースで使われている。
1 つのコードの説明は 1 か所にしかない。領域の文字は文書のファイル名と合い、重大度は番号の範囲と合う（``crates/cli/tests/error_codes.rs``）。
インストーラ（``install.sh``）のコードは ``scripts/test-install-codes.sh`` が同じ対応を確かめる。

設定キーは、キーごとに設定ファイルの章にラベル ``cfg-<表の名前>-<キー>`` を持つ。
設定のキーと文書のラベルの対応は ``crates/cli/tests/config_keys_documented.rs`` が確かめる。

.. _page-dev-build-release:

ビルドとリリース
----------------

.. _sec-dev-build:

ソースのビルドとテスト
~~~~~~~~~~~~~~~~~~~~~~

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
~~~~~~~~~~~~~~~~~~~~

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

.. csv-table::
   :delim: ¦
   :header-rows: 1
   :widths: 30 70

   ソフトウェア¦用途
   Rust（``cargo``）¦図のデータを作る本体のビルドと実行
   LuaLaTeX（luatexja）と dvisvgm¦TikZ 図（HTML 用の SVG と PDF 用の PDF）
   inkscape¦ロゴ（SVG）を PDF に変換する（PDF 版のみ）
   upLaTeX、dvipdfmx、latexmk¦PDF 版の組版。TeX Live なら ``texlive-lang-japanese`` など日本語関連のパッケージを含めて入れる
   IPAex フォント¦図と PDF 本文の日本語

PDF の組版中に LaTeX がエラーで止まったときは、``doc/build/latex/ignisyeet.log`` を開き、``!`` で始まる行とその直後の行番号を確認する。
多くはパッケージ不足かフォント不足である。原因を直したあと、もう一度 ``make pdf`` を実行すればよい。

ページ名を変えるときは、``conf.py`` の ``redirects`` に元の URL と新しいページの対応を書く。転送ではアンカーは引き継がれない。

.. _sec-dev-doc-scripts:

図とデータのスクリプト
~~~~~~~~~~~~~~~~~~~~~~

``doc/scripts/`` のスクリプトは、ドキュメントの図とデータを作る。保存済みのデータは ``doc/data/`` にある。

.. csv-table:: ``doc/scripts/`` のスクリプト
   :delim: ¦
   :header-rows: 1
   :widths: 34 66

   スクリプト¦内容
   ``build_tikz.py``¦``doc/figures/tikz/*.tex`` を SVG と PDF にする
   ``make_plots.py``¦matplotlib の図を作る（本体の出力と ``doc/data/`` を読む）
   ``make_screens.py``¦端末の出力を疑似端末で受け、画面を PNG にする
   ``make_ogp.py``¦リンクの共有用画像を作る
   ``extract_cfd_fields.py``、``cfd_axial_split.py``¦終わった CFD の実行から、図に使う流れの場とフィンの面ごとの軸力を取り出す
   ``basic_finner_cfd.py``、``basic_finner_compare.py``¦Basic Finner の CFD と係数表を、実測と比べる
   ``rans_*.py``¦RANS の検証の実行と集計（平板、円錐、サンプル機体、メッシュ、粗さ補正）
   ``perf_profile.sh``¦各段階の実行時間とプロファイルを測る（結果は ``doc/data/perf/``）

.. _sec-development:

CI とリリース
~~~~~~~~~~~~~

**リリースとバージョン。** IgnisYeet のバージョンは ``Cargo.toml`` の ``[workspace.package] version``\ （``ignisyeet --version`` で表示される値）で決まり、
GitHub では同じ番号のタグ ``vX.Y.Z`` と GitHub Release で管理する。インストーラの user は、
``--version`` を指定しなければ最新のリリースを入れる。

**CI。** push とプルリクエストでは、ワークフロー ``ci.yml`` が ``cargo build --release --locked`` と ``cargo test --release --locked`` を実行する。
続けて、例題で ``geom``、``aero``、``sim``、風の格子の ``dispersion``、モンテカルロの ``dispersion``、``python/plot.py`` を通しで動かし、
``python/plot.py`` の単体テスト（``python/test_plot.py``）を実行し、``sim.max_time`` を短くした飛翔が終了ステータス 3 で終わることを確かめる。
clippy は ``-D warnings``、rustfmt は ``--check`` で実行し、違反があるとジョブが失敗する。
ShellCheck（``install.sh`` と ``scripts/*.sh``）も、指摘があるとジョブが失敗する。誤検出は、理由を書いた ``# shellcheck disable=SCxxxx`` の行で個別に除く。
別のジョブが、必要な最低の Rust の版 1.87 で ``cargo build --locked`` を実行する。

ワークフロー ``docs.yml`` は、プルリクエストと ``main`` への push で HTML のドキュメントを警告をエラーとして（``-W``）ビルドする。
GitHub Pages への公開は ``main`` への push（と手動の実行）のときだけで、プルリクエストではビルドだけを行う。
ワークフローが使う Action は、コミットの SHA で固定している。

**リリース。** タグ ``v*`` を push すると、ワークフロー ``release.yml`` が次の処理を行う。

#. タグと ``Cargo.toml`` のバージョンが一致することを確かめる（一致しなければ失敗する）。
#. 4 つのターゲット向けにバイナリを作る。Linux は musl で静的にリンクするので、ディストリビューションによらず動く。
   ``x86_64-unknown-linux-musl`` と ``aarch64-unknown-linux-musl`` は Ubuntu（cargo-zigbuild）で作り、前者だけテストを実行する（後者はクロスビルド）。
   ``x86_64-apple-darwin``\ （Intel）と ``aarch64-apple-darwin``\ （Apple Silicon）は macOS で作り、テストを実行する。
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
