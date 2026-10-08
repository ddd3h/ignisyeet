.. _page-reference-config-environment:

環境の設定
============

.. _cfg-earth:

``[earth]``
-----------

積分する座標系と重力モデルを指定する。

.. _cfg-earth-model:

``earth.model``
^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"flat"``
:値/範囲: ``"flat"`` ``"ecef"``
:説明: 積分する座標系。``"flat"`` は射点の局所 ENU 座標系（平面地球・慣性系）、``"ecef"`` は WGS84 楕円体に固定した回転座標系（Coriolis 力と遠心力を含む）。
:関連: :ref:`sec-ecef`、:ref:`page-theory-atmosphere-gravity`

.. _cfg-earth-gravity:

``earth.gravity``
^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"inverse_square"``
:値/範囲: ``"constant"`` ``"inverse_square"`` ``"j2"``
:説明: 重力モデル。``"constant"`` は :math:`g_0` 一定、``"inverse_square"`` は逆 2 乗則、``"j2"`` は点質量に :math:`J_2` 項を加える。``"j2"`` は ``earth.model = "ecef"`` のときだけ指定できる。
:関連: :ref:`sec-gravity`、:ref:`IY-C049`

.. _cfg-atmosphere:

``[atmosphere]``
----------------

大気モデルを指定する。この設定は飛翔計算と摩擦抗力の Reynolds 数（射点高度の大気）に使われる。

.. _cfg-atmosphere-model:

``atmosphere.model``
^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"us1976"``
:値/範囲: ``"us1976"`` ``"constant"``
:説明: ``"us1976"`` は米国標準大気 1976、``"constant"`` は高度によらず一定の大気。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-atmosphere-temperature_offset:

``atmosphere.temperature_offset``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.0``
:値/範囲: 有限、-186.946 より大きい
:説明: 標準大気の温度に一律に足す温度差 :math:`\Delta T` [K]。``us1976`` のみ。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-atmosphere-density:

``atmosphere.density``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.225``
:値/範囲: 有限で正
:説明: 密度 [kg/m\ :sup:`3`]。``constant`` のみ。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-atmosphere-sound_speed:

``atmosphere.sound_speed``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``340.29``
:値/範囲: 有限で正
:説明: 音速 [m/s]。``constant`` のみ。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-atmosphere-viscosity:

``atmosphere.viscosity``
^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.789e-5``
:値/範囲: 有限で正
:説明: 粘性係数 [Pa s]。``constant`` のみ。
:関連: :ref:`page-theory-atmosphere-gravity`

.. _cfg-wind:

``[wind]``
----------

風速の高度分布を指定する。

.. _cfg-wind-model:

``wind.model``
^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"power"``
:値/範囲: ``"constant"`` ``"power"`` ``"log"`` ``"profile"``
:説明: 風速の高度分布。``"constant"`` は一定風、``"power"`` はべき法則、``"log"`` は対数則、``"profile"`` は高度別の表。
:関連: :ref:`page-theory-wind`、:ref:`sec-wind`

.. _cfg-wind-speed:

``wind.speed``
^^^^^^^^^^^^^^

:型: 数値
:既定値: ``4.0``
:値/範囲: 0 以上
:説明: 基準高度での風速 [m/s]。``profile`` では使わない（表の風速を使う。``profile`` で書いてあると警告する）。
:関連: :ref:`page-theory-wind`

.. _cfg-wind-direction_deg:

``wind.direction_deg``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.0``
:値/範囲: 0 以上 360 以下
:説明: 風が吹いてくる方位 [deg]（北から時計回り）。``profile`` では使わない（表の風向を使う。``profile`` で書いてあると警告する）。
:関連: :ref:`page-theory-wind`

.. _cfg-wind-ref_height:

``wind.ref_height``
^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``2.0``
:値/範囲: 正
:説明: 風速の基準高度 [m]。
:関連: :ref:`page-theory-wind`、:ref:`IY-C051`

.. _cfg-wind-exponent:

``wind.exponent``
^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``6.0``
:値/範囲: 正
:説明: べき法則の指数 :math:`n`。``power`` のみ。
:関連: :ref:`page-theory-wind`

.. _cfg-wind-roughness_length:

``wind.roughness_length``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.03``
:値/範囲: 0 より大きく ``ref_height`` の半分より小さい
:説明: 粗度長 :math:`z_0` [m]。``log`` のみ。
:関連: :ref:`page-theory-wind`、:ref:`IY-C052`

.. _cfg-wind-ground_blend_height:

``wind.ground_blend_height``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.0``
:値/範囲: 正で ``ref_height`` より小さい
:説明: 地面付近で速度を 0 になめらかにつなぐ高さ [m]。``power`` と ``log`` のみ。``log`` では :math:`z_0` より大きい必要があり、設定値が :math:`z_0` 以下なら :math:`\max(\text{値}, 2 z_0)` に読み替える。
:関連: :ref:`page-theory-wind`、:ref:`IY-C053`

.. _cfg-wind-profile:

``wind.profile``
^^^^^^^^^^^^^^^^

:型: パス
:既定値: なし
:説明: ``model = "profile"`` のとき必須。高度別の風の CSV ファイル。
:関連: :ref:`sec-wind-profile`、:ref:`IY-C050`

.. _cfg-wind-profile_mean_speed:

``wind.profile_mean_speed``
^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: なし
:値/範囲: 有限で 0 以上
:説明: ``profile`` のみ。表の平均風速 :math:`W` をこの値 [m/s] にするよう、表のすべての風速を同じ倍率で拡大・縮小する。省略すると表の風速のまま。
:関連: :ref:`sec-wind-scaling`、:ref:`IY-C057`

.. _cfg-wind-profile_mean_direction_deg:

``wind.profile_mean_direction_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: なし
:値/範囲: 0 以上 360 以下
:説明: ``profile`` のみ。表の平均風の風向 :math:`\Psi` がこの方位 [deg] になるよう、表のすべての風向を同じ角度だけ回す。省略すると表の風向のまま。
:関連: :ref:`sec-wind-scaling`、:ref:`IY-C058`

