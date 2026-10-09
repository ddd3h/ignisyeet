.. _page-theory:

理論と解説
============

設定の選択肢を選ぶための根拠と、モデルの式を述べる。

この部の読み方
--------------

設定ファイルの値を決めるときに読む部である。:ref:`page-theory-limitations` を除く各章は、同じ六つの部分でできている。

1. **この章で決めること**：対応する設定キーと値を最初に挙げる。
2. **選び方**：判断表または流れ図。選択肢ごとに、表すもの、仮定、有効範囲、精度の根拠、計算コスト、選ぶとき・避けるときを比べる。
3. **モデル**：何を計算するか。式、仮定、記号を述べる。処理の手順は書かない。
4. **適用範囲と限界**：数値で書ける境界と、仮定が崩れる条件。:ref:`page-theory-limitations` の節へ案内する。
5. **精度の証拠**：検証・相互比較・妥当性確認の結果。:ref:`page-vv` の該当節へ案内する。
6. **関連する設定キー**：キー、既定値、効果の向き。値の完全な定義は :ref:`page-reference-config` にある。

数値を挙げた箇所には、条件（Mach 数、迎角、機体、メッシュ）を付けた。
「検証」はモデルと解析解、手計算、収束との比較、「相互比較」は手法どうしの比較、「妥当性確認」は実測との比較を指す。

選択マップ
----------

設定ブロックごとの選択肢と、それを説明する章を :numref:`fig-opt-map` に示す。
青の見出しは機体と環境の入力、橙の見出しは空力・飛翔・落下分散の計算手法である。

.. _fig-opt-map:

.. figure:: /_generated/tikz/opt_map.*
   :width: 100%

   設定ブロックと選択肢、それを説明する章の対応。見出しは設定ファイルの表（``[wind]`` など）、中の項目は選ぶキーと値、下の箱が読む章である。項目は主なキーだけで、全体は :numref:`tbl-theory-choose-map` にある。

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
     - 表 :numref:`tbl-choose-environment`、結果への効き方 :numref:`tbl-atmosphere-effect`
   * - ``earth.gravity``
     - :ref:`page-theory-atmosphere-gravity`
     - 表 :numref:`tbl-choose-environment`、結果への効き方 :numref:`tbl-gravity-effect`
   * - ``wind.model``、``ref_height``、``exponent``、``roughness_length``
     - :ref:`page-theory-wind`
     - 流れ図 :numref:`fig-choose-wind-model`、風速の比 :numref:`tbl-wind-ratio`、結果への効き方 :numref:`tbl-wind-effect`
   * - ``wind.profile_mean_speed``、``profile_mean_direction_deg``、``ground_blend_height``、``dispersion.wind_speeds``、``dispersion.directions``
     - :ref:`page-theory-wind`
     - 平均風速の拡大と着地点 :numref:`tbl-wind-profile-scale`、風速格子の値の意味（``profile`` と他のモデル）
   * - ``rocket.dry_mass``、``cg_dry``、``ixx_dry``、``iyy_dry``、``motor.*``
     - :ref:`page-theory-propulsion`
     - 質量特性の入力を変えたときの結果の変化 :numref:`tbl-choose-propulsion`
   * - ``rocket.cg_lateral_dry``、``ixy_dry`` など
     - :ref:`page-theory-propulsion`、:ref:`page-theory-dynamics`
     - 質量特性の表 :numref:`tbl-choose-propulsion`、重心の横ずれと静的余裕 :numref:`tbl-choose-margin`
   * - ``rocket.fin_le``、``fin_te``、``fin_cant_deg``、``roughness``、``extra_cd``、``fin_override``
     - :ref:`page-theory-barrowman`
     - 入力を変えたときの頂点高度・着地点・静的余裕の変化 :numref:`tbl-choose-barrowman-sens`
   * - ``aero.method``
     - :ref:`page-theory-aero-choose`
     - 流れ図 :numref:`fig-choose-aero-method`、状況別 :numref:`tbl-choose-aero-situation`、手法の比較 :numref:`tbl-aero-methods`、範囲 :numref:`tbl-choose-aero-range`、根拠 :numref:`tbl-choose-aero-evidence`
   * - ``aero.table_kind``、``mach_step``、``alpha_step_deg``、``extrapolation``、``general_mach_step``
     - :ref:`page-theory-aero-tables`
     - 流れ図 :numref:`fig-choose-table-kind`、表 :numref:`tbl-choose-table-kind`、格子の刻み :numref:`tbl-choose-table-grid`
   * - ``aero.panel.*``
     - :ref:`page-theory-panel`
     - メッシュ :numref:`tbl-choose-panel-mesh`、遷音速の帯 :numref:`tbl-choose-panel-transonic`。``fin_section`` は選べる値が 1 つ
   * - ``aero.cfd.*``
     - :ref:`page-theory-cfd-choose`
     - 流れ図 :numref:`fig-choose-cfd-model`、:numref:`fig-choose-cfd-surface`、表 :numref:`tbl-choose-cfd-model`、資源 :numref:`tbl-cfd-parallel`
   * - ``sim.integrator``、``sim.attitude``、``sim.dt``、``rtol``、``atol``
     - :ref:`page-theory-integrators`
     - 流れ図 :numref:`fig-integrator-choice`、表 :numref:`tbl-choose-integrator`、:numref:`tbl-choose-attitude`
   * - ``sim.output_interval``、``sim.max_time``
     - :ref:`page-theory-integrators`
     - 刻み幅と許容誤差の節（:numref:`tbl-integrator-accuracy`）。出力間隔は結果をほとんど変えない
   * - ``sim.quasi_steady``
     - :ref:`page-theory-dynamics`
     - 図 :numref:`fig-quasi-steady-schematic`、表 :numref:`tbl-choose-quasi-steady`
   * - ``sim.descent``、``recovery.*``
     - :ref:`page-theory-descent`
     - 図 :numref:`fig-descent-modes`、表 :numref:`tbl-choose-descent`
   * - ``dispersion.mode``
     - :ref:`page-theory-dispersion`
     - 流れ図 :numref:`fig-dispersion-choice`
   * - ``dispersion.monte_carlo.*``（``samples``、``seed``、``truncate_sigma``、標準偏差）
     - :ref:`page-theory-dispersion`
     - 標準偏差 :numref:`tbl-mc-perturb`、サンプル数 :numref:`tbl-safety-samples`

結果を使う前に、仮定と適用範囲の一覧 :ref:`page-theory-limitations` を確かめる。

章の一覧
--------

.. toctree::
   :maxdepth: 1

   overview
   geometry
   coordinates
   propulsion
   aero_choose
   aero_coefficients
   aero_tables
   barrowman
   panel
   dynamics
   descent
   integrators
   limitations
