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
   * - :doc:`own_rocket`
     - 自分の機体の STL と質量特性を設定し、形状の抽出を確かめる
   * - :doc:`motor`
     - 推力曲線（``.eng``）を用意して機体に載せる
   * - :doc:`launcher_parachute`
     - 射点、ランチャ、パラシュート（単一・二段開傘）を設定する
   * - :doc:`environment`
     - 風、大気、地球のモデルを設定する。高度別の風の表を作る
   * - :doc:`external_table`
     - 風洞試験などの外部の係数表を使う
   * - :doc:`dispersion_run`
     - 風の格子とモンテカルロで落下分散を計算して読む
   * - :doc:`keepout_zone`
     - 保安距離（保安域の半径）を求める
   * - :doc:`sweeps`
     - 設定を書き換えずに条件を変えて繰り返す
   * - :doc:`read_results`
     - 端末の表示と ``summary.json`` を読み、結果が使えるかを確かめる
   * - :doc:`plots_kml`
     - 図を描き、Google Earth（KML）で見る
   * - :doc:`warnings_errors`
     - ワーニング、エラー、終了ステータス 3 に対処する

.. toctree::
   :maxdepth: 1

   install
   own_rocket
   motor
   launcher_parachute
   environment
   external_table
   dispersion_run
   keepout_zone
   sweeps
   read_results
   plots_kml
   warnings_errors
