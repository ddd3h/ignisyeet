.. _page-reference-config-dispersion:

落下分散の設定
================

.. _cfg-dispersion:

``[dispersion]``
----------------

落下分散の計算方式を指定する。

.. _cfg-dispersion-mode:

``dispersion.mode``
^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"wind_grid"``
:値/範囲: ``"wind_grid"`` ``"monte_carlo"``
:説明: ``"wind_grid"`` は風速 × 風向の格子、``"monte_carlo"`` はモンテカルロ解析。
:関連: :ref:`page-theory-dispersion`

.. _cfg-dispersion-wind_speeds:

``dispersion.wind_speeds``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列
:既定値: ``[1, 2, 3, 4, 5, 6, 7]``
:値/範囲: 1 個以上 100 個以下、各要素は 0 以上
:説明: ``wind_grid`` で計算する風速の一覧 [m/s]。``constant`` ``power`` ``log`` では基準高度 ``wind.ref_height`` の風速、``profile`` では表の平均風速。
:関連: :ref:`sec-wind-scaling`、:ref:`sec-dispersion-safety`、:ref:`IY-C040`、:ref:`IY-C043`

.. _cfg-dispersion-directions:

``dispersion.directions``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``8``
:値/範囲: 1 以上 360 以下
:説明: ``wind_grid`` の風向の分割数。北から等間隔に分ける。風の格子のケース数（``wind_speeds`` の数 × ``directions``）は 100000 以下。
:関連: :ref:`IY-C041`、:ref:`IY-C042`、:ref:`IY-C044`

.. _cfg-dispersion-monte_carlo:

``[dispersion.monte_carlo]``
----------------------------

``dispersion.mode = "monte_carlo"`` のときに使う。

各ばらつきは互いに独立な正規分布 :math:`N(0,\sigma^2)` から引き、このキーの値は標準偏差 :math:`\sigma` である。0 はそのばらつきを無効にする。負の値や有限でない値はエラーである。"相対" は公称値に対する割合（0.03 は 3 %。公称値に :math:`1+\text{乱数}` を掛け、下限は 0.05）、"絶対" は公称値に加える量である。

.. _cfg-dispersion-monte_carlo-samples:

``dispersion.monte_carlo.samples``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``1000``
:値/範囲: 1 以上 1e6 以下
:説明: サンプル数。
:関連: :ref:`sec-mc`、:ref:`IY-C045`、:ref:`IY-C046`

.. _cfg-dispersion-monte_carlo-seed:

``dispersion.monte_carlo.seed``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``1``
:値/範囲: 0 以上
:説明: 乱数の種。サンプル :math:`i` は ``seed + i`` を種とする。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-thrust_scale:

``dispersion.monte_carlo.thrust_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.03``
:値/範囲: 0 以上
:説明: 相対。推力（全力積が比例して変わる。推進剤質量は変えない）。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-burn_time_scale:

``dispersion.monte_carlo.burn_time_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.02``
:値/範囲: 0 以上
:説明: 相対。燃焼時間の伸縮（推力曲線の時間軸を伸ばす）。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-dry_mass:

``dispersion.monte_carlo.dry_mass``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.1``
:値/範囲: 0 以上
:説明: 相対。乾燥質量。乾燥慣性テンソルのすべての要素に同じ倍率を掛ける。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-cg:

``dispersion.monte_carlo.cg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.01``
:値/範囲: 0 以上
:説明: 絶対 [m]。乾燥重心の位置（正が後方）。慣性モーメントは変えない。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-cn_scale:

``dispersion.monte_carlo.cn_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.10``
:値/範囲: 0 以上
:説明: 相対。法線力係数 :math:`C_N`、:math:`C_{N\alpha}`、ピッチ減衰。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-ca_scale:

``dispersion.monte_carlo.ca_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.15``
:値/範囲: 0 以上
:説明: 相対。軸力（抗力）係数 :math:`C_A`\ （燃焼中・燃焼後）。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-elevation_deg:

``dispersion.monte_carlo.elevation_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.5``
:値/範囲: 0 以上
:説明: 絶対 [deg]。レールの向きの設置誤差 :math:`\sigma_{\mathrm{tilt}}`。公称の軸を、面内（射角）と面に垂直な 2 方向の独立な傾きで等方に傾ける。機体はレールとともに回る。傾けた射角が :math:`(0,90]` の外のサンプルは失敗になる。
:関連: :ref:`sec-mc-rail`

.. _cfg-dispersion-monte_carlo-azimuth_deg:

``dispersion.monte_carlo.azimuth_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.0``
:値/範囲: 0 以上
:説明: 絶対 [deg]。射方位の設定誤差。傾けたレールを機体ごと鉛直軸の周りに回す（水平方向の効果は :math:`\sigma\cos(\text{射角})`）。
:関連: :ref:`sec-mc-rail`

.. _cfg-dispersion-monte_carlo-wind_speed:

``dispersion.monte_carlo.wind_speed``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.0``
:値/範囲: 0 以上
:説明: 絶対 [m/s]。基準風速（``wind.speed`` の周りに分布し、0 未満は 0 にする）。``profile`` では表の平均風速 :math:`W`\ （``profile_mean_speed`` を指定したときはその値）の周りの揺らぎで、表を :math:`\max(W+\delta,0)/W` 倍する。表がほぼ無風（平均 0.1 m/s 未満）でこの値が 0 でないときはエラーである。
:関連: :ref:`sec-wind-scaling`

.. _cfg-dispersion-monte_carlo-wind_direction_deg:

``dispersion.monte_carlo.wind_direction_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``15.0``
:値/範囲: 0 以上
:説明: 絶対 [deg]。風向（``wind.direction_deg`` の周りに分布する。``profile`` では表の平均風の風向の周り。``profile_mean_direction_deg`` を指定したときはその方位の周り）。
:関連: :ref:`sec-wind-scaling`

.. _cfg-dispersion-monte_carlo-parachute_cd_s_scale:

``dispersion.monte_carlo.parachute_cd_s_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.1``
:値/範囲: 0 以上
:説明: 相対。パラシュートの抗力面積 :math:`C_DS`。``recovery.main_cd_s`` にも同じ倍率を掛ける。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-thrust_misalignment_deg:

``dispersion.monte_carlo.thrust_misalignment_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 絶対 [deg]。推力の傾き。:math:`y_b`、:math:`z_b` 方向の傾き成分に独立な正規乱数を足し、傾きの角度と方位を決める。
:関連: :ref:`sec-thrust-line`

.. _cfg-dispersion-monte_carlo-thrust_offset:

``dispersion.monte_carlo.thrust_offset``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 絶対 [m]。ノズル出口の点。:math:`y_b`、:math:`z_b` 方向に独立な正規乱数を足す。
:関連: :ref:`sec-thrust-line`

.. _cfg-dispersion-monte_carlo-fin_cant_deg:

``dispersion.monte_carlo.fin_cant_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 絶対 [deg]。設定したフィンのカントに加える量。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-cg_lateral:

``dispersion.monte_carlo.cg_lateral``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 絶対 [m]。乾燥重心の横ずれ。:math:`y_b`、:math:`z_b` 方向に独立な正規乱数を足す。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-cl_scale:

``dispersion.monte_carlo.cl_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 相対。ロールのモーメント係数 :math:`C_l` とカントの傾き :math:`C_{l\delta}`。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-clp_scale:

``dispersion.monte_carlo.clp_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 相対。ロール減衰微係数 :math:`C_{lp}`。
:関連: :ref:`sec-mc`

.. _cfg-dispersion-monte_carlo-lag_scale:

``dispersion.monte_carlo.lag_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 相対。遅れの和 ``lag_l0``、``lag_l1``\ （胴体とフィンの見かけの質量の遅れと対流の回転の項）。
:関連: :ref:`sec-aero-unsteady`、:ref:`sec-quasi-steady`

.. _cfg-dispersion-monte_carlo-magnus_scale:

``dispersion.monte_carlo.magnus_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 相対。Magnus 力とモーメントの傾き ``cnpa``、``cmpa``。組み込みの手法はこれらを推算しないので、外部の表で値を与えたときだけ効く（値がなければ警告する）。
:関連: :ref:`sec-aero-unsteady`

.. _cfg-dispersion-monte_carlo-cmq_scale:

``dispersion.monte_carlo.cmq_scale``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上
:説明: 相対。ピッチ・ヨーの減衰の和（減衰のモーメントと回転による法線力）。``cn_scale`` の倍率に重ねて掛ける。見かけの質量の項には掛からない。
:関連: :ref:`sec-quasi-steady`

.. _cfg-dispersion-monte_carlo-truncate_sigma:

``dispersion.monte_carlo.truncate_sigma``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: なし
:値/範囲: 有限で正
:説明: 指定すると、引いたすべての標準正規乱数 :math:`z` を :math:`[-k,k]`\ （:math:`k` はこの値）に切ってから標準偏差を掛ける。省略すると切らない正規分布である。
:関連: :ref:`sec-mc`、:ref:`IY-C032`


``thrust_misalignment_deg`` から ``cmq_scale`` までの 9 項目は既定で無効である。
乱数は、ロールと非対称の 6 項目を最初の 11 項目のあとに、空力の 3 項目（``lag_scale`` ``magnus_scale`` ``cmq_scale``）をさらにそのあとに引く。
したがって、最初の 11 項目の値はこの 9 項目の設定に依存せず、空力の 3 項目の設定は、ほかのどの項目の値も変えない。
