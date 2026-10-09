.. _page-howto:

使い方
========

目的別の手順を並べる。各ページは 1 つの目的を扱う。

目的ごとのページを次の表に示す。

.. list-table:: 目的とページ
   :header-rows: 1
   :widths: 30 70

   * - ページ
     - できること
   * - :doc:`install`
     - インストールして動作を確かめる
   * - :doc:`cfd_setup`
     - CFD に必要なツール（SU2、gmsh、MPI）を用意して確かめる
   * - :doc:`cfd_table`
     - CFD で係数表を作り、結果を確かめ、中断から再開する
   * - :doc:`cad_step`
     - CAD のモデル（STEP）から CFD の係数表を作る
   * - :doc:`read_results`
     - 端末の表示と ``summary.json`` を読み、結果が使えるかを確かめる
   * - :doc:`plots_kml`
     - 図を描き、Google Earth（KML）で見る
   * - :doc:`warnings_errors`
     - ワーニング、エラー、終了ステータス 3 に対処する

.. toctree::
   :maxdepth: 1

   install
   cfd_setup
   cfd_table
   cad_step
   read_results
   plots_kml
   warnings_errors
