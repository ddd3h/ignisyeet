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
   * - :doc:`cfd_setup`
     - CFD に必要なツール（SU2、gmsh、MPI）を用意して確かめる
   * - :doc:`cfd_table`
     - CFD で係数表を作り、結果を確かめ、中断から再開する
   * - :doc:`cad_step`
     - CAD のモデル（STEP）から CFD の係数表を作る
   * - :doc:`dispersion_run`
     - 風の格子とモンテカルロで落下分散を計算して読む
   * - :doc:`keepout_zone`
     - 保安距離（保安域の半径）を求める
   * - :doc:`sweeps`
     - 設定を書き換えずに条件を変えて繰り返す

.. toctree::
   :maxdepth: 1

   install
   own_rocket
   motor
   launcher_parachute
   environment
   external_table
   cfd_setup
   cfd_table
   cad_step
   dispersion_run
   keepout_zone
   sweeps
