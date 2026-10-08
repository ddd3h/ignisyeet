:og:description: IgnisYeet は、ロケットの 3D モデル（STL）から空力特性を推算し、6 自由度の剛体運動の飛翔シミュレーションと落下分散解析までを一貫して行うオープンソースの飛翔シミュレータである。

.. meta::
   :description: IgnisYeet は、ロケットの 3D モデル（STL）から空力特性を推算し、6 自由度の剛体運動の飛翔シミュレーションと落下分散解析までを一貫して行うオープンソースの飛翔シミュレータである。

IgnisYeet ドキュメント
======================

.. image:: _generated/logo/IgnisYeet-logo.*
   :width: 260px
   :align: center
   :alt: IgnisYeet logo

**IgnisYeet** は、ロケットの 3D モデル（STL ファイル）から空力特性を推算し、
6 自由度の剛体運動の飛翔シミュレーションと落下分散解析までを一貫して行うオープンソースの飛翔シミュレータである。

* STL を読み込み、機体の外形（胴体の断面形状とフィン）を自動で取り出す。
* 取り出した形状から、Mach 数と迎角の格子上で空力係数を **一度だけ** 計算して保存する。
* 保存した係数表を内挿・外挿しながら、ランチャ滑走から着地までを 6 自由度の剛体運動として数値積分する。
* 風速と風向を組み合わせた全ケースを並列に計算し、落下分散を求める。

計算の重い部分は Rust、作図は Python で実装している。

.. _page-index-entry:

読者別の入口
------------

本書は、学ぶ・行う・引く・理解する・信頼するの五つの部でできている。
各部の関係を :numref:`fig-doc-map` に、目的ごとの入口を次の表に示す。

.. _fig-doc-map:

.. figure:: /_generated/tikz/doc_map.*
   :width: 90%

   本書の五つの部と、読み進める向き。矢印は「次に読む先」を表す。

.. list-table:: 目的と入口
   :header-rows: 1
   :widths: 46 54

   * - したいこと
     - 読む部
   * - 初めて使う。まず動かす
     - :ref:`page-tutorial`
   * - 自分の機体、推力曲線、風、CFD、落下分散を設定して実行する
     - :ref:`page-howto`
   * - 設定キー、コマンド、出力ファイルの列、エラーコード、記号を引く
     - :ref:`page-reference`
   * - 設定の選択肢を選ぶ根拠、モデルの式、適用範囲を知る
     - :ref:`page-theory`
   * - 結果をどこまで信頼できるかを知る
     - :ref:`page-vv`
   * - ビルド、テスト、リリース、ソースの構成を知る
     - :ref:`page-dev`
   * - 開発の方針、ライセンス、記法と用語を知る
     - :ref:`page-about`

.. toctree::
   :maxdepth: 2
   :numbered:
   :caption: 目次

   about
   tutorial/index
   howto/index
   reference/index
   theory/index
   vv/index
   dev/index

.. raw:: latex

   \setcounter{secnumdepth}{-1}
   \renewcommand{\chaptermark}[1]{\markboth{#1}{}}

.. toctree::
   :maxdepth: 1
   :caption: 付録

   references
