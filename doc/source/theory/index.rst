.. _page-theory:

理論と解説
============

設定の選択肢を選ぶための根拠と、モデルの式を述べる。

この部の読み方
--------------

設定ファイルの値を決めるときに読む部である。各章は同じ六つの部分でできている。

1. **この章で決めること**：対応する設定キーと値を最初に挙げる。
2. **選び方**：判断表または流れ図。選択肢ごとに、表すもの、仮定、有効範囲、精度の根拠、計算コスト、選ぶとき・避けるときを比べる。
3. **モデル**：何を計算するか。式、仮定、記号を述べる。処理の手順は書かない。
4. **適用範囲と限界**：数値で書ける境界と、仮定が崩れる条件。:ref:`page-theory-limitations` の節へ案内する。
5. **精度の証拠**：検証・相互比較・妥当性確認の結果。:ref:`page-vv` の該当節へ案内する。
6. **関連する設定キー**：キー、既定値、効果の向き。値の完全な定義は :ref:`page-reference-config` にある。

数値を挙げた箇所には、条件（Mach 数、迎角、機体、メッシュ）を付けた。
「検証」はモデルと基準解または別のモデルとの比較、「妥当性確認」は実測との比較を指す。

選択マップ
----------

設定ブロックごとの選択肢と、それを説明する章を :numref:`fig-opt-map` に示す。
青の見出しは機体と環境の入力、橙の見出しは空力・飛翔・落下分散の計算手法である。

.. _fig-opt-map:

.. figure:: /_generated/tikz/opt_map.*
   :width: 100%

   設定ブロックと選択肢、それを説明する章の対応。見出しは設定ファイルの表（``[wind]`` など）、中の項目は選ぶキーと値、下の箱が読む章である。

選び方の一覧
------------

各設定の判断材料（流れ図または表）がどの章にあるかを :numref:`tbl-theory-choose-map` に示す。

.. _tbl-theory-choose-map:

.. list-table:: 設定の選択肢と判断材料の置き場
   :header-rows: 1
   :widths: 30 22 48

   * - 設定キー（値）
     - 章
     - 判断材料
   * - ``rocket.stl_scale``、``nose_direction``、``fin_override``、``aero.n_slices``、``aero.fin_threshold``
     - :ref:`page-theory-geometry`
     - 表 :numref:`tbl-choose-geometry`、断面数としきい値の影響 :numref:`tbl-geometry-slices`、:numref:`tbl-geometry-threshold`、形状の特徴と抽出の振る舞い :numref:`tbl-geometry-shapes`
   * - ``earth.model``
     - :ref:`page-theory-coordinates`
     - 流れ図 :numref:`fig-choose-earth-model`、表 :numref:`tbl-choose-earth`、平面地球が使える範囲 :numref:`fig-enu-error`
   * - ``atmosphere.model``、``temperature_offset``
     - :ref:`page-theory-atmosphere-gravity`
     - 表 :numref:`tbl-choose-atmosphere`、結果への効き方 :numref:`tbl-atmosphere-effect`
   * - ``earth.gravity``
     - :ref:`page-theory-atmosphere-gravity`
     - 表 :numref:`tbl-choose-gravity`、結果への効き方 :numref:`tbl-gravity-effect`
   * - ``wind.model``、``ref_height``、``exponent``、``roughness_length``、``profile_mean_*``
     - :ref:`page-theory-wind`
     - 流れ図 :numref:`fig-choose-wind-model`、表 :numref:`tbl-choose-wind`、風速の比 :numref:`tbl-wind-ratio`、結果への効き方 :numref:`tbl-wind-effect`
   * - ``rocket.dry_mass``、``cg_dry``、``ixx_dry``、``iyy_dry``、``motor.*``
     - :ref:`page-theory-propulsion`
     - 質量特性の入力を変えたときの結果の変化 :numref:`tbl-choose-propulsion`
   * - ``aero.method``、``aero.table_kind``、``aero.panel.*``、``aero.cfd.*``
     - :ref:`page-theory-aero-choose`、:ref:`page-theory-aero-tables`、:ref:`page-theory-panel`、:ref:`page-theory-cfd-choose`
     - 各章の「選び方」
   * - ``sim.integrator``、``sim.attitude``、``sim.quasi_steady``、``sim.descent``、``recovery.*``
     - :ref:`page-theory-integrators`、:ref:`page-theory-dynamics`、:ref:`page-theory-descent`
     - 各章の「選び方」
   * - ``dispersion.mode``、``dispersion.monte_carlo.*``
     - :ref:`page-theory-dispersion`
     - 各章の「選び方」

結果を使う前に、仮定と適用範囲の一覧 :ref:`page-theory-limitations` を確かめる。

章の一覧
--------

.. toctree::
   :maxdepth: 1

   overview
   geometry
   coordinates
   atmosphere_gravity
   wind
   propulsion
   aero_choose
   aero_coefficients
   aero_tables
   barrowman
   panel
   cfd_choose
   cfd_equations
   cfd_coefficients
   dynamics
   descent
   integrators
   dispersion
   limitations
