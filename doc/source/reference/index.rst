.. _page-reference:

リファレンス
==============

コマンド、設定、ファイル、コードを引く。判断や助言は書かず、選び方は理論と解説に置く。

引くものごとのページを次の表に示す。

.. list-table:: 引くものとページ
   :header-rows: 1
   :widths: 30 70

   * - ページ
     - 載っているもの
   * - :doc:`config/index`
     - 設定ファイルの形式、値の検査、各表のキー（基本、空力、CFD、環境、飛翔計算、落下分散）
   * - :doc:`installer`
     - ``install.sh`` のオプションと環境変数
   * - :doc:`aero_table_format`
     - 係数表（CSV と付随 JSON）、``aero_drag.csv``、``panel_cp.csv``
   * - :doc:`cfd_output_files`
     - ``cfd_cases.csv`` と ``cfd_report.json``
   * - :doc:`symbols`
     - 記号、単位、座標系の符号

.. toctree::
   :maxdepth: 1

   config/index
   installer
   aero_table_format
   cfd_output_files
   symbols
