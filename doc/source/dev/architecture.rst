.. _page-dev-architecture:

構成
======

リポジトリの構成を述べる。クレートの依存、データの流れ、ディレクトリ、規約の順に読む。

.. _sec-dev-crates:

クレート構成
--------------

計算は Rust のワークスペース（``crates/``）の 6 つのクレートで行い、作図は Python（``python/plot.py``）が受け持つ。
:numref:`fig-arch-crates` は、クレートの依存とデータの流れである。

.. _fig-arch-crates:

.. figure:: /_generated/tikz/arch_crates.*
   :width: 80%

   クレートの依存（実線の矢印は依存する向き）と、出力ファイルから作図へのデータの流れ（破線）。

.. list-table:: クレートの役割
   :header-rows: 1
   :widths: 14 86

   * - クレート
     - 役割
   * - ``geom``
     - STL の入出力、機軸の決定、断面の切り出し、フィンの抽出、サンプル形状
   * - ``aero``
     - 標準大気、部品積み上げ法の空力モデル、係数表（保存・内挿・外挿）
   * - ``panel``
     - パネル法の空力解析（亜音速の Morino 法、超音速の局所傾斜法）
   * - ``cfd``
     - CFD による空力解析（gmsh のメッシュ、SU2 のケースの実行、係数表の作成）
   * - ``sim``
     - 推力曲線、6 自由度の飛翔と積分法、風・大気・重力、座標変換、落下分散
   * - ``cli``
     - ``ignisyeet`` コマンド、設定ファイル、出力ファイル、端末の表示

.. _sec-dev-flow:

データの流れ
--------------

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
------------------

リポジトリの主なファイルとディレクトリを :numref:`fig-directory-tree` に示す。

.. _fig-directory-tree:

.. figure:: /_generated/tikz/directory_tree.*
   :width: 100%

   リポジトリの構成。フォルダのアイコンはディレクトリ、紙のアイコンはファイルを表す。

.. _sec-dev-error-codes:

エラーコードとドキュメントの対応
----------------------------------

エラーとワーニングのコード ``IY-<領域><番号>`` は、コード 1 つにつき 1 つのページ（``doc/source/errors/<領域>.rst`` の節）を持つ。
領域の文字とファイルの対応は ``C``\ （config）、``G``\ （geometry）、``A``\ （aero）、``S``\ （sim）、``O``\ （cli）、``F``\ （cfd）、``I``\ （install）である。
番号は 001〜499 がエラー、500〜999 がワーニングである。

次の対応はテストが確かめる。ソースに現れるコードはすべて文書にあり、文書にあるコードはすべてソースで使われている。
1 つのコードの説明は 1 か所にしかない。領域の文字は文書のファイル名と合い、重大度は番号の範囲と合う（``crates/cli/tests/error_codes.rs``）。
インストーラ（``install.sh``）のコードは ``scripts/test-install-codes.sh`` が同じ対応を確かめる。

設定キーは、キーごとに ``reference/config/`` のページにラベル ``cfg-<表の名前>-<キー>`` を持つ。
設定のキーと文書のラベルの対応は ``crates/cli/tests/config_keys_documented.rs`` が確かめる。
