推力と質量特性
==============

推力の物理
----------

ロケットの推力は、ノズルから噴出するガスの運動量と、ノズル出口の圧力差から生じる :cite:`sutton`。

.. math::
   :label: eq-thrust-theory

   T = \dot m\,v_e + (p_e-p_\infty)\,A_e

:math:`\dot m` は推進剤の質量流量、:math:`v_e` は噴出速度、:math:`p_e` と :math:`A_e` はノズル出口の圧力と面積、:math:`p_\infty` は周囲の圧力である。
比推力 :math:`I_{sp}` を用いれば :math:`T=I_{sp}\,g_0\,\dot m` とも書ける。

しかし、燃焼室の圧力履歴や噴出速度を精度よく予測するのは難しい。
IgnisYeet では多くのシミュレータと同様に、燃焼試験などで **実測した推力履歴** をそのまま用いる。
:eq:`eq-thrust-theory` の第 2 項の高度依存（上空で推力がわずかに増える効果）は考慮していない。

推力曲線（RASP 形式）
---------------------

推力履歴はモデルロケットで広く使われる RASP 形式 :cite:`rasp`\ （``.eng``）で与える。

.. code-block:: text

   ; コメント行
   ; 名前  直径[mm] 長さ[mm] 遅延  推進剤質量[kg] 全備質量[kg] 製造者
   IY-SAMPLE 75 500 0 3.0 4.5 IgnisYeet
   0.00 0
   0.05 2300
   0.20 2400
   ...
   3.10 0

1 行目がヘッダ、2 行目以降が「時刻 [s] と推力 [N]」の組である。
時刻 :math:`t_k` の間は線形補間し、最後の時刻を燃焼終了時刻 :math:`t_b` とする。
先頭が :math:`t=0` でない場合は :math:`(0,0)` を補う。

推進剤の消費
------------

燃焼中の質量流量は推力に比例する（比推力が一定）と仮定する :cite:`sutton`。
すると時刻 :math:`t` までに消費した推進剤は、その時刻までの力積に比例する。

.. math::
   :label: eq-prop-mass

   I(t) = \int_0^t T(t')\,\dd t',\qquad
   m_p(t) = m_{p0}\left(1-\frac{I(t)}{I_t}\right),\qquad I_t=I(t_b)

:math:`I_t` は全力積、:math:`m_{p0}` は初期の推進剤質量である。:math:`I(t)` は推力曲線の台形積分で厳密に求まる。
サンプルのモータ（架空）の推力と推進剤質量を :numref:`fig-thrust-mass` に示す。

.. _fig-thrust-mass:

.. figure:: _generated/plots/thrust_mass.*
   :width: 100%

   サンプルモータの推力曲線（左）と推進剤質量（右）。

質量・重心・慣性モーメント
--------------------------

推進剤を、モータケースと同じ直径 :math:`D_m=2r_m`、長さ :math:`L_m` の一様な円柱とみなす。
その重心はモータの中央 :math:`x_p=x_{\mathrm{aft}}-L_m/2` にある（:math:`x_{\mathrm{aft}}` はモータ後端の位置）。
乾燥質量 :math:`m_d`、乾燥重心 :math:`x_d`、乾燥慣性モーメント :math:`I_{xx,d}, I_{yy,d}` と合わせて

.. math::
   :label: eq-mass-props

   \begin{aligned}
   m &= m_d + m_p,\qquad x_{cg} = \frac{m_d\,x_d + m_p\,x_p}{m},\\
   I_{yy} &= I_{yy,d} + m_d\,(x_d-x_{cg})^2 + m_p\,\frac{3r_m^2+L_m^2}{12} + m_p\,(x_p-x_{cg})^2,\\
   I_{xx} &= I_{xx,d} + \frac12\,m_p\,r_m^2
   \end{aligned}

とする。:math:`I_{yy}` は重心まわりのピッチ・ヨーの慣性モーメントで、平行軸の定理 :cite:`goldstein` で現在の重心まわりに移している。
軸対称なので :math:`I_{zz}=I_{yy}` である。

推進剤の消費による運動量の変化（いわゆるジェットダンピング）や、質量変化率 :math:`\dot m` による慣性モーメントの時間微分の項は無視している。
