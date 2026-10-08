.. _page-dev:

開発者向け
============

IgnisYeet のソースの構成、ビルドとテスト、ドキュメントの作成、リリースを述べる。
利用だけなら、この部は読まなくてよい。

.. toctree::
   :maxdepth: 1

   architecture
   build_release

.. _sec-dev-notes:

実装メモの置き場
----------------

利用者向けのページには、モデルと観測できる挙動だけを書く。計算の手順、ファイルの扱い、内部の定数といった実装の説明は、ソースコードのコメントに置く。
コメントは ``cargo doc --no-deps --document-private-items`` で読める（Python は docstring）。
:numref:`tbl-dev-notes` は、話題ごとの置き場である。

.. _tbl-dev-notes:

.. list-table:: 実装メモの置き場
   :header-rows: 1
   :widths: 40 60

   * - 話題
     - 置き場
   * - 設定の読み込み、設定の同一性の判定、係数表の再利用の条件
     - ``crates/cli/src/config.rs``、``crates/aero/src/table.rs``
   * - 出力ファイルの書き出し
     - ``crates/cli/src/outfile.rs``
   * - KML の出力
     - ``crates/cli/src/kml.rs``
   * - 作図（``ignisyeet-plot``）
     - ``python/plot.py`` の docstring
   * - STL の読み込みと形状の抽出
     - ``crates/geom/src/stl.rs``、``crates/geom/src/extract.rs``
   * - 部品積み上げ法の係数表
     - ``crates/aero/src/model.rs``、``crates/aero/src/table.rs``
   * - パネル法のメッシュ、影響行列、解法
     - ``crates/panel/src/mesh.rs``、``influence.rs``、``solver.rs``、``supersonic.rs``
   * - CFD の SU2 設定ファイル
     - ``crates/cfd/src/su2cfg.rs``、``crates/cfd/src/su2_writer.py``
   * - CFD の壁面（STL、STEP、対称性、忠実度）
     - ``crates/cfd/src/surface.rs``、``stl_mesh.rs``、``step_mesh.rs``、``symmetry.rs``、``fidelity.rs``、``mesh_stl.py``、``mesh_step.py``、``stl_union.py``
   * - CFD の体積メッシュと境界層
     - ``crates/cfd/src/mesh.rs``、``mesh_gmsh.py``、``bl_layer.py``
   * - CFD のケースの実行、収束の判定、再開
     - ``crates/cfd/src/runner.rs``、``history.rs``、``case.rs``
   * - CFD の力の積分と係数表の作成
     - ``crates/cfd/src/forces.rs``、``table.rs``、``general.rs``、``asymmetry.rs``
   * - 運動方程式と数値積分、ステップ内の事象
     - ``crates/sim/src/flight.rs``、``stepper.rs``、``integrator.rs``
   * - 落下分散の乱数とサンプルの割り当て
     - ``crates/sim/src/dispersion.rs``

エラーコードとドキュメントの対応は :ref:`sec-dev-error-codes` にある。
