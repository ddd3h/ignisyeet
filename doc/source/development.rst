.. _page-dev:

開発者向け
==========

IgnisYeet のソースの構成、ビルドとテスト、ドキュメントの作成、リリース、各リリースの履歴を述べる。
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

設定キーは、キーごとに設定ファイルの章にラベル ``cfg-<表の名前>-<キー>`` を持つ。
コードと文書の対応、設定キーと文書のラベルの対応は、テストが確かめる。その内容は :ref:`sec-testing-codes` にある。

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

テストの種類、実行のしかた（SU2、gmsh、MPI を使うテストを含む）、確かめる内容は :ref:`page-testing` にある。

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
   ``cost_measure.py``¦各段階の時間、CPU 時間、メモリ、スレッド数への依存を測る（結果は ``doc/data/cost/``。:ref:`page-cost` の表と図の元になる）

.. _sec-dev-reproduce:

検証の再現
~~~~~~~~~~

:ref:`page-vv` の式と数値解法の検証は、リポジトリの直下で ``cargo test --release`` を実行すると再現できる。
SU2 を使う比較はテストに含まれず、CFD ツールを入れた環境で別に実行した結果である。どのテストが何を確かめるかは :ref:`page-testing` にある。

CFD と RANS の計算結果は ``doc/data/cfd/`` に、Basic Finner の実測値、計算の出力、比較の表と各数値の出典は ``doc/data/validation/basic_finner/`` に、計算時間とメモリの測定は ``doc/data/perf/`` と ``doc/data/cost/`` に保存してある。
:ref:`page-vv-validation` の表と図は、これらの値から作る。
Basic Finner の計算に使った設定と形状は ``examples/basic_finner/`` にある。
サンプル機体の係数表と飛翔の結果は、``examples/`` の設定ファイルを :ref:`page-tutorial` の手順で実行して得る。

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

.. _sec-release-history:

リリース履歴
------------

v0.1.0 から v0.4.0 までの実装の違いを、リポジトリの記録（タグとコミット）から整理する。
v0.1.0 は C++ の試作であり、v0.2.0 以降は Rust で書き直した現在の実装である。

:numref:`tbl-release-compare` は、四つのリリースの比較である。
行数は、リポジトリ内の Rust（``.rs``）、C++（``.cpp``、``.hpp``。同梱の外部ライブラリ ``toml.hpp`` を除く）、Python（``.py``）のファイルの全行を数えたもので、テスト、空行、コメントを含む。
ドキュメントは数えない。

.. _tbl-release-compare:

.. list-table:: リリースごとの比較
   :header-rows: 1
   :widths: 16 21 21 21 21

   * - 項目
     - v0.1.0
     - v0.2.0
     - v0.3.0
     - v0.4.0
   * - 日付
     - 2025-06-10
     - 2026-10-04
     - 2026-10-04
     - 2026-10-13
   * - 言語
     - C++17
     - Rust と Python
     - Rust と Python
     - Rust と Python
   * - 構成
     - ヘッダとソースの群（Makefile）
     - 5 クレート
     - 6 クレート
     - 6 クレート
   * - 本体の行数
     - 約 6,600（C++）
     - 約 9,200（Rust）
     - 約 15,000（Rust）
     - 約 42,800（Rust）
   * - Python の行数
     - 0
     - 約 1,200
     - 約 2,200
     - 約 11,800
   * - テスト
     - 試験用プログラム 5 本
     - ``#[test]`` 73 件
     - ``#[test]`` 157 件
     - ``#[test]`` 594 件
   * - 形状の入力
     - なし
     - STL
     - STL
     - CAD（STEP、IGES、BREP）または STL
   * - 空力
     - 設定で与える係数
     - 部品積み上げ法、パネル法、外部の表
     - 前のリリースに加え CFD（SU2）
     - 前のリリースに加え六成分の準定常空力
   * - CFD
     - なし
     - なし
     - Euler。RANS は未検証
     - Euler と RANS。CAD の壁面
   * - 積分法
     - Euler、leapfrog、RK4、RK45
     - RK4、RK45
     - RK4、RK45、DOP853
     - RK4、RK45、DOP853
   * - 降下
     - パラシュートの抗力の計算
     - 弾道、パラシュート
     - 弾道、パラシュート
     - 弾道、パラシュート、二段開傘
   * - 落下分散
     - 宣言のみ
     - 風の格子、モンテカルロ
     - 風の格子、モンテカルロ
     - 前のリリースに加え保安距離の手順
   * - ドキュメント
     - 開発メモのみ
     - rst 14 ファイル
     - rst 16 ファイル
     - rst 36 ファイル

v0.1.0（C++、2025-06-10）
~~~~~~~~~~~~~~~~~~~~~~~~~

**実装。** C++17 で書いた 6 自由度の飛翔計算の試作である。``Makefile`` と ``CMakeLists.txt`` でビルドし、物理（ベクトル、クォータニオン、状態、重力、大気）、シミュレーションエンジン、機体、出力の各部を ``include/`` と ``src/`` に分ける。
設定は TOML（``parameter.toml``）、出力は CSV、JSON Lines、バイナリである。同梱の ``toml.hpp`` を除くソースは約 6,600 行で、試験用のプログラムが 5 本ある（重力、大気、最小構成）。
``src/`` には ``main.cpp`` のほかに試作段階の ``main_old.cpp``、``main_new.cpp``、``main_v3.cpp``、``main_v3_simple.cpp`` が残っており、エントリポイントは一つにまとまっていない。

**主な機能。** 重力は一様、高度依存、自転する地球の三段階、大気は定数、標準大気、風と突風を含む動的な大気の三段階から選ぶ。積分法は Euler、leapfrog、RK4、適応刻みの RK45 である。
空力は、設定ファイルで与える係数から力とモーメントを求める基本的なモデルである。STL のパスは設定の項目にあるが、形状の読み込みは実装されていない。落下分散の解析は宣言だけで、実装がない。

v0.2.0（Rust への書き直し）
~~~~~~~~~~~~~~~~~~~~~~~~~~~

**実装。** Rust のワークスペースに書き直し、C++ のコードは引き継いでいない。クレートは ``geom``、``aero``、``panel``、``sim``、``cli`` の 5 つで、作図は Python（``python/plot.py``）が受け持つ。
Rust は約 9,200 行、Python は約 1,200 行、``#[test]`` は 73 件である。ライセンスは GPL-3.0-or-later で、Sphinx による日本語のドキュメント（rst 14 ファイル）を備える。

**前のリリースからの新しい点。**

* 入力が、パラメータファイルから、機体の STL、質量特性、推力曲線（RASP の ``.eng``）、射場の条件になった。機軸の決定、胴体の半径分布、フィンの検出を STL から行う。
* 空力係数を Mach 数と迎角の格子で一度だけ計算して表にし、飛翔ではこの表を内挿・外挿する。方法は部品積み上げ法（Barrowman 法）、パネル法、外部の表の三つである。
* 平らな地球と自転する地球（ECEF）、一様・逆二乗・J2 の重力、標準大気 1976、一定・べき乗・対数・高度表の風を扱う。
* 落下分散を、風速と風向の格子とモンテカルロ法（1σ と 3σ の誤差楕円）で並列に求め、弾道落下とパラシュート降下の両方で評価する。KML を出力する。
* コマンドは ``geom``、``aero``、``sim``、``dispersion``、``sample-stl`` である。

v0.3.0
~~~~~~

**実装。** v0.2.0 の構成に ``cfd`` クレートを加えた 6 クレートである。Rust は約 15,000 行、Python は約 2,200 行、``#[test]`` は 157 件で、ドキュメントは rst 16 ファイルである。

**前のリリースからの新しい点。**

* CFD による係数表（``aero.method = "cfd"``）を加えた。機体の形状を OpenCASCADE の立体に作り直し、gmsh でメッシュを切り、SU2 の Euler 方程式で Mach 数と迎角の行列を解く。RANS も選べるが、このリリースでは検証していない。``ignisyeet cfd-check`` で CFD のツールを確かめる。
* 積分法に DOP853 を、姿勢の積分に Lie 群の方法を加えた。地面の近くの風の滑らかな接続も加えた。
* 計算に使うスレッド、メモリ、優先度を ``[resources]`` で制限できる。
* ``curl | bash`` で入れるインストーラ（``install.sh``）と、タグごとのリリース用アーカイブの作成を加えた。

v0.4.0
~~~~~~~~~~~~~~~~

**実装。** クレートの構成は v0.3.0 と同じ 6 つである。Rust は約 42,800 行、Python は約 11,800 行、``#[test]`` は 594 件で、ドキュメントは rst 36 ファイル（付録のエラーコード 8 ファイルを含む）である。
ドキュメントは、利用、設定、手順、モデル、検証と妥当性確認を順に読む一冊の構成に改めた。

**前のリリースからの新しい点。**

* 機体の入力に CAD のファイル（``rocket.cad``。STEP、IGES、BREP）を加えた。STL（``rocket.stl``）も使える。CFD では、CAD の B-rep をそのまま壁面にできる（``surface = "step"``）。
* RANS を、プリズム層のメッシュ、軸力の補正、収束の判定とともに使える形にした。RANS の検証の範囲は、平板、円すい、フィンなし胴体、サンプル機体のメッシュの収束と Euler との相互比較である。
* 六成分の準定常空力を加えた。減衰、見かけ質量、Magnus 力、誘導ロール、フィンの取り付け角、ロール共振の確認を含み、横滑り角を含む係数表を使う。
* 降下に二段開傘（ドローグとメイン）を加えた。
* 落下分散に、着地距離の百分位数（P99.9）、乱数の打ち切り（``dispersion.monte_carlo.truncate_sigma``）、保安距離の手順を加えた。
* Basic Finner を、STEP と STL の形状、計算の設定とともに ``examples/basic_finner/`` に加え、超音速の範囲で実測と比べた（妥当性確認）。ほかの比較は手法どうしの相互比較である。
* ``--set KEY=VALUE`` と ``--out DIR`` を ``geom``、``aero``、``sim``、``dispersion`` に加えた。
* エラーとワーニングに ``IY-<領域><番号>`` のコードを付け、コードごとに原因と対処のページを設けた。
* インストーラは、``--cfd`` で SU2 を実行する CPU 向けにコンパイルするようにした（conda-forge のバイナリは ``--su2-binary``）。
