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
