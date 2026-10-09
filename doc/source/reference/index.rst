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
   * - :doc:`cli`
     - ``ignisyeet`` のサブコマンドとオプション、端末の出力モード、``make`` と ``plot.py``
   * - :doc:`installer`
     - ``install.sh`` のオプションと環境変数
   * - :doc:`output_files`
     - 出力ファイルの一覧と、形状・飛翔・落下分散の列と項目
   * - :doc:`aero_table_format`
     - 係数表（CSV と付随 JSON）、``aero_drag.csv``、``panel_cp.csv``
   * - :doc:`cfd_output_files`
     - ``cfd_cases.csv`` と ``cfd_report.json``
   * - :doc:`exit_codes`
     - 終了ステータス 0、1、2、3 の定義
   * - :doc:`error_codes`
     - エラーコードとワーニングコードの体系と、コード別の説明
   * - :doc:`symbols`
     - 記号、単位、座標系の符号

.. toctree::
   :maxdepth: 1

   cli
   installer
   output_files
   aero_table_format
   cfd_output_files
   exit_codes
   error_codes
   symbols
