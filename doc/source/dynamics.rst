6 自由度の運動方程式
====================

状態量
------

飛翔中の状態を次の 13 個の量で表す。

.. math::

   \bm{y} = \left(\bm{x},\ \bm{v},\ \bm{q},\ \bm{\omega}\right)

:math:`\bm{x}` と :math:`\bm{v}` は ENU 座標での重心の位置と速度、:math:`\bm{q}` は姿勢のクォータニオン、
:math:`\bm{\omega}` は機体座標での角速度である。時刻 :math:`t` は点火を 0 とする。
以下の式は既定の平面地球モード（``earth.model = "flat"``）のものである。
``"ecef"`` では :math:`\bm{x},\bm{v}` が ECEF 座標での位置と地球に対する速度、:math:`\bm{\omega}` が ECEF に対する角速度になり、
Coriolis 力と遠心力の項が加わる以外は同じ式を使う（:ref:`sec-ecef`）。
重力の大きさとして ``constant`` と ``inverse_square`` が与えるのは遠心力を含む有効重力で、ECEF ではそこから万有引力を求めて遠心項を別に足すので、正味の重力は平面地球モードと同じ有効重力になる（:ref:`sec-ecef-gravity`）。``j2`` は万有引力そのものを使う。

並進運動
--------

重心の運動は Newton の運動方程式に従う :cite:`goldstein`。

.. math::
   :label: eq-translation

   \dot{\bm{x}} = \bm{v},\qquad
   \dot{\bm{v}} = \frac{1}{m(t)}\,R(\bm{q})\left(\bm{F}_T + \bm{F}_A\right)_b + \bm{g}(z_U)

:math:`(\cdot)_b` は機体座標で表した量であることを示す。推力 :math:`\bm{F}_T` の向きと作用線は :ref:`sec-thrust-line` で述べる。
ミスアライメントとノズルの偏心がなければ、推力は機軸方向で重心を通り、

.. math::

   \bm{F}_{T,b} = \left(T(t),\,0,\,0\right)^\top

である。推進剤の消費による質量の変化は :math:`m(t)` を通じて入る。:math:`\dot m\,\bm{v}` の項が現れないのは、推力 :math:`T` が実測値として
噴出ガスの運動量変化をすでに含んでいるからである :cite:`sutton`。

.. _sec-aero-forces:

空気力
~~~~~~

係数表には **軸対称表** と **一般表** があり、設定 ``aero.table_kind`` で選ぶ（概念は :ref:`sec-aero-table`、係数の定義は :doc:`aero_coefficients`）。
飛翔計算は、表の種類に応じて次の二つの式で空気力と空力モーメントを求める。

対気速度を機体座標で表したものを :math:`\bm{u}=R^\top(\bm{v}-\bm{w})=(u_x,u_y,u_z)` とし、

.. math::
   :label: eq-alpha

   V=|\bm{u}|,\qquad u_\perp=\sqrt{u_y^2+u_z^2},\qquad
   \alpha_t=\operatorname{atan2}(u_\perp,\,u_x),\qquad M=\frac{V}{a(h_0+z_U)}

とする。:math:`\alpha_t` はピッチとヨーを合わせた **全迎角**\ （:math:`0\le\alpha_t\le\pi`）である。
機体座標での迎角 :math:`\alpha` と横滑り角 :math:`\beta` は :ref:`sec-aero-angles` で定義する。

軸対称表
^^^^^^^^

機体は軸対称なので、法線力は迎角の面内（機軸と対気速度を含む面）にあり、横向きの対気速度 :math:`(u_y,u_z)` と逆向きに働く。
係数表から :math:`(M,\alpha_t')`\ （:math:`\alpha_t'=\min(\alpha_t,\pi-\alpha_t)`）で係数を引き、:math:`q=\tfrac12\rho V^2` として

.. math::
   :label: eq-aero-force

   \bm{F}_{A,b} =
   -C_A\,q\,S\,\sgn(u_x)\begin{pmatrix}1\\0\\0\end{pmatrix}
   -C_N\,q\,S\,\frac{1}{u_\perp}\begin{pmatrix}0\\u_y\\u_z\end{pmatrix}

とする。燃焼中（:math:`T>0`）は :math:`C_A^{\mathrm{on}}`、燃焼後は :math:`C_A^{\mathrm{off}}` を使う。
:math:`\sgn(u_x)` により、機体が後ろ向きに飛ぶとき（:math:`u_x<0`）も軸力は常に対気速度に逆らう向きになる。

.. _sec-aero-general:

一般表
^^^^^^

一般表は、機体座標の力とモーメントの 6 成分を、Mach 数 :math:`M`、迎角 :math:`\alpha`、横滑り角 :math:`\beta` の関数として持つ。
係数の定義は :ref:`sec-aero-coeff6` にある。
飛翔中の :math:`(\alpha,\beta)` は :ref:`sec-aero-angles` の式で対気速度から求め、表を三線形内挿で引く。
軸対称表と違って後ろ向きの飛行を鏡像にせず、:math:`\alpha=\pm180^\circ` 付近も表の一部として持つ。
空気力は

.. math::
   :label: eq-aero-force6

   \bm{F}_{A,b} = q\,S\begin{pmatrix}C_X\\C_Y\\C_Z\end{pmatrix}

である。:math:`C_X` は燃焼中（:math:`T>0`）に :math:`C_X^{\mathrm{on}}`、燃焼後に :math:`C_X^{\mathrm{off}}` を使う。
表のモーメント係数はノーズ先端まわりの値なので、重心まわりのモーメントには、ノーズ先端から重心へ移すことで生じる力のモーメントを足す。
重心の機体座標は、ノーズ先端を基準にして :math:`(-x_{cg},\,y_{cg},\,z_{cg})` と書ける（:math:`y_{cg},z_{cg}` は重心の横方向のずれで、軸上なら 0）。
したがって、重心から見たノーズ先端の位置は :math:`(x_{cg},\,-y_{cg},\,-z_{cg})` で、

.. math::
   :label: eq-aero-moment6

   \bm{M}_{A,b} = q\,S\,d\begin{pmatrix}C_l^{\mathrm{tot}}\\C_m\\C_n\end{pmatrix}
   + \begin{pmatrix}x_{cg}\\-y_{cg}\\-z_{cg}\end{pmatrix}\times\bm{F}_{A,b}

とする。ロールの係数 :math:`C_l^{\mathrm{tot}}` は :ref:`sec-roll-moment` で述べる。
軸対称な機体を一般表に展開すると、:math:`y_{cg}=z_{cg}=0` のとき\ :eq:`eq-aero-moment6` のピッチ成分は :math:`q\,S\,(x_{cp}-x_{cg})\,C_Z` になり、
軸対称表の復元モーメント（:eq:`eq-moment-static`）に一致する。

一般表では、法線力の作用点を 1 つの圧力中心では表せない。
``trajectory.csv`` の ``x_cp``\ （その状態の迎角での圧力中心の表示）には、横方向の力 :math:`(C_Y,C_Z)` とそのモーメントから求めた **等価な圧力中心**

.. math::
   :label: eq-xcp-eff

   x_{cp}=\frac{d\,(C_mC_Z-C_nC_Y)}{C_Y^2+C_Z^2}

を使う。:math:`C_m=x_{cp}C_Z/d,\ C_n=-x_{cp}C_Y/d` から逆算した値で、横方向の力が 0 に近いときは迎角 :math:`0.5^\circ` の値で代える。
この値は、横方向の力の向きで重み付けしたピッチとヨーの圧力中心の平均にあたり、ピッチ面とヨー面の圧力中心が違う機体では、その間の値になる。
静的な安定の判定には、この値ではなく、次に述べる小迎角の余裕を使う。

.. _sec-static-margin-general:

静安定余裕の判定
~~~~~~~~~~~~~~~~

静的な安定は、軸対称表でも一般表でも、迎角 0 の極限での静安定余裕（小迎角の余裕）で判定する。

* 軸対称表：飛翔計算が使う補間した表（:ref:`sec-aero-interp`）から、最初の迎角の区間で求める。
  :math:`\delta` を :math:`0.5^\circ` と迎角の格子の 0 でない最小の角度の小さいほうとし、全迎角 :math:`\delta` での法線力 :math:`C_N(\delta)` と圧力中心 :math:`x_{cp}(\delta)`\ （Mach 数について内挿した :math:`C_N` とノーズ先端まわりのモーメント :math:`C_Nx_{cp}` の比、:eq:`eq-xcp-interp`）を、その状態の Mach 数で引く。
  法線力の傾き :math:`S=-C_N(\delta)/\delta` と復元の微係数 :math:`R=C_N(\delta)\bigl(x_{cp}(\delta)-x_{cg}\bigr)/(d\,\delta)` から、一般表と同じく :eq:`eq-margin-pitch-yaw` で余裕を求める。
  :math:`C_N(\delta)>0` なら

  .. math::
     :label: eq-margin-axisym

     \mathrm{SM}=\frac{x_{cp}(\delta)-x_{cg}}{d}

  で、ピッチ面とヨー面の余裕は同じ値である。
  表は :math:`C_N(0)=0` なので、最初の区間では :math:`C_N` と :math:`C_Nx_{cp}` が迎角に比例し、:math:`x_{cp}(\delta)` は最初の 0 でない迎角の行の圧力中心（Mach 数について内挿した値）に等しい。
  これは、小さな迎角で飛翔計算が法線力を加える点そのものである。表の迎角 0 の行の ``xcp`` と ``cna`` の列は、余裕に入らない。
  同じ表を展開した一般表も、同じ余裕を与える。
  :math:`C_N(\delta)=0` では余裕は求まらず（``the normal-force slope dCN/dalpha is zero at Mach 0.10``）、
  :math:`C_N(\delta)<0` では、角度を広げるモーメントなら負の余裕 :math:`R/|S|`\ （不安定）、復元するモーメントなら求まらない
  （``the normal-force slope dCN/dalpha is negative at Mach 0.10 (the normal force points along the crossflow, a restoring moment cannot be expressed as a margin; check the sign convention of the table)``。Mach 数は例）。
  部品積み上げ法、パネル法、CFD の表の :math:`C_N` は小さな正の迎角で通常正で、求まらないのは主に外部の表の符号や値の誤りのときである。
* 一般表（6 成分の表）：ピッチ面とヨー面の余裕を、迎角 0 と横滑り角 0 での表の傾きから別々に求め、小さいほうで判定する（以下）。

その状態が飛んでいる迎角は、判定に入らない。迎角が大きいと胴体揚力（:eq:`eq-body-lift`）で圧力中心が前に動くが、これはその迎角での力の作用点であり、機体の静的な安定の指標ではない。
迎角を入れないので、頂点付近の遅くて迎角の大きい弧や、強い横風の中のランチャ離脱を、不安定と数えることはない。
``trajectory.csv`` の ``x_cp`` と ``stability_cal`` は、その状態の迎角での圧力中心と :math:`(x_{cp}-x_{cg})/d` の表示で（一般表では :eq:`eq-xcp-eff` の等価な圧力中心）、判定には使わない。

以下は一般表の余裕の求め方である。
表の符号の約束（:ref:`sec-aero-table`）では、正の迎角で :math:`C_Z=-C_N<0`、正の横滑り角で :math:`C_Y=-C_N<0` で、ノーズ先端まわりのモーメントは :math:`C_m=x_{cp}C_Z/d`、:math:`C_n=-x_{cp}C_Y/d` である。
重心 :math:`x_{cg}` まわりのモーメント係数は

.. math::
   :label: eq-cm-cg

   C_{m,cg}=C_m-\frac{x_{cg}}{d}\,C_Z,\qquad C_{n,cg}=C_n+\frac{x_{cg}}{d}\,C_Y

である。静的な安定は、角度を戻す向きのモーメントの傾き（復元の微係数）:math:`R` の符号で決まる。
ピッチ面（:math:`\beta=0`）とヨー面（:math:`\alpha=0`）で、:math:`R` と法線力の傾き :math:`S` を

.. math::
   :label: eq-margin-restoring

   R_{\mathrm{pitch}}=-\frac{\partial C_{m,cg}}{\partial\alpha}\bigg|_{\beta=0},\quad S_{\mathrm{pitch}}=\frac{\partial C_Z}{\partial\alpha}\bigg|_{\beta=0},\qquad
   R_{\mathrm{yaw}}=\frac{\partial C_{n,cg}}{\partial\beta}\bigg|_{\alpha=0},\quad S_{\mathrm{yaw}}=\frac{\partial C_Y}{\partial\beta}\bigg|_{\alpha=0}

とし（:math:`R>0` が安定）、余裕を

.. math::
   :label: eq-margin-pitch-yaw

   \mathrm{SM}=\frac{R}{|S|}\quad[\mathrm{cal}]

とする。物理的な符号 :math:`S<0` では、これは :math:`\mathrm{SM}_{\mathrm{pitch}}=(x_{cp,\mathrm{pitch}}-x_{cg})/d`、:math:`\mathrm{SM}_{\mathrm{yaw}}=(x_{cp,\mathrm{yaw}}-x_{cg})/d` に等しい
（:math:`x_{cp,\mathrm{pitch}}/d=(\partial C_m/\partial\alpha)/(\partial C_Z/\partial\alpha)`、:math:`x_{cp,\mathrm{yaw}}/d=-(\partial C_n/\partial\beta)/(\partial C_Y/\partial\beta)`）。
:math:`S>0`\ （法線力が横流れと同じ向きを指す）でも、:math:`R<0`\ （角度を広げるモーメント）なら負の余裕 :math:`R/|S|`\ （不安定）とする。
:math:`S>0` で :math:`R\ge0` のときは、復元するモーメントを圧力中心の位置で表せないので、その面の余裕は求まらない（NaN）。
:math:`|S|\le10^{-12}`\ （法線力の傾きが 0）の面も求まらない。理由は、それぞれ
``the pitch normal-force slope dCz/dalpha is zero at Mach 0.10`` と
``the yaw normal-force slope dCy/dbeta is positive at Mach 0.10 (the normal force points along the crossflow, a restoring moment cannot be expressed as a margin; check the sign convention of the table)``
の形（面と Mach 数は例）で残す。
傾きは、角度 0 の前後 :math:`\pm\delta` での中心差分で、:math:`\delta` は :math:`0.5^\circ` と、その軸の格子の 0 でない最小の角度の小さいほうである。
格子が 0 を含めば、どちらも補間した表の 0 での傾きを与える。
判定に使う余裕は :math:`\min(\mathrm{SM}_{\mathrm{pitch}},\mathrm{SM}_{\mathrm{yaw}})` で、ピッチとヨーの連成は考えない。

同じ表を同じ格子に展開した一般表は、軸対称表と同じ余裕を与える。どちらも最初の迎角の区間の割線の傾きから求めるからである。
ただし、既定の一般表は、軸対称表とは別の格子で作る（:ref:`sec-aero-table`）。既定の軸対称表は迎角 1° 刻みで、最初の区間は 0〜1° だが、展開した一般表の迎角と横滑り角の格子は :math:`\pm30^\circ` まで 2° 刻み（その外は 10° 刻み）で、最初の格子点は 2° である。
区間が広いほど、その区間で胴体揚力が圧力中心を前に動かす分を含むので、一般表の余裕のほうが小さく出る。
サンプルのランチャ離脱（Mach 0.13）では、軸対称表で 1.99 cal、展開した一般表で 1.89 cal と、約 0.1 cal の差がある。この差は格子の細かさの違いによるもので、差は安全側である。

余裕は、飛翔計算そのものの中で、すべての飛翔（``sim``、``dispersion`` の公称の飛翔、風の格子のケース、モンテカルロのサンプル）について求める。
自由飛行のうち頂点より前で対気速度が 10 m/s を超える間の、受け入れた積分のステップの始端ごとと、ランチャ離脱の時刻に、その状態の Mach 数と重心で 2 つの余裕を求める。
ランチャ離脱の時刻の余裕は、対気速度によらず求め、離脱時の対気速度が 10 m/s を超えるときは自由飛行の最小値にも 1 回だけ入れる。
軌道の点を記録するかどうか（``sim.output_interval``）には依らない。

* ``min_stability_cal`` と ``stability_at_rail_exit`` は、その時刻のピッチとヨーの余裕の小さいほう（求まったもの）の、それぞれ最小値とランチャ離脱時の値である。頂点までに対気速度が 10 m/s を超えない飛翔では ``min_stability_cal`` は ``null`` で、判定（``static_margin_min_cal``）にはランチャ離脱時の値を使う。
* ``static_margin_pitch_min_cal`` と ``static_margin_yaw_min_cal`` は、面ごとの最小値（自由飛行とランチャ離脱を合わせたもの）である。どこかで余裕が求まらなかった面は ``null`` になる。
* 余裕が求まらない時刻があれば、最初のものを ``at t = 0.50 s: …`` の形で ``stability_unknown`` に残し、静的な安定は「判定できない」とする（:ref:`sec-exit-status`）。求まった余裕に負のものがあれば、判定できない余裕があっても不安定とする。
* 最小の余裕をとった状態の時刻と Mach 数を ``static_margin_min_t`` と ``static_margin_min_mach`` に残す（余裕を一度も求めなかった飛翔では ``null``）。
  結果パネルの ``min margin`` の行、プレーンモードの ``min static margin`` の行、静安定余裕の警告と ``RESULTS NOT USABLE`` の文には、これを ``at t = 0.25 s, Mach 0.13`` の形で添える（数はサンプルの値）。

回転運動
--------

機体座標での角速度の変化は Euler の運動方程式に従う :cite:`goldstein`。
慣性テンソル :math:`I` は、現在の重心まわりで機体座標に対して表した対称行列である。

.. math::
   :label: eq-euler

   I\,\dot{\bm{\omega}} = \bm{M}_b - \bm{\omega}\times\left(I\bm{\omega}\right),\qquad
   \dot{\bm{q}} = \frac12\,\bm{q}\otimes(0,\bm{\omega})

である。第 2 項はジャイロ効果を表す。:math:`\bm{M}_b` は空力モーメントと推力のモーメント（:ref:`sec-thrust-line`）の和である。

.. _sec-inertia-tensor:

慣性テンソル
~~~~~~~~~~~~

慣性テンソルは **テンソルの行列要素** で与える。

.. math::
   :label: eq-inertia-tensor

   I=\begin{pmatrix}I_{xx}&I_{xy}&I_{xz}\\I_{xy}&I_{yy}&I_{yz}\\I_{xz}&I_{yz}&I_{zz}\end{pmatrix},\qquad
   \begin{aligned}
   I_{xx}&=\int(y^2+z^2)\,\mathrm{d}m, & I_{xy}&=-\int xy\,\mathrm{d}m,\\
   I_{yy}&=\int(z^2+x^2)\,\mathrm{d}m, & I_{xz}&=-\int xz\,\mathrm{d}m,\\
   I_{zz}&=\int(x^2+y^2)\,\mathrm{d}m, & I_{yz}&=-\int yz\,\mathrm{d}m
   \end{aligned}

非対角要素は、慣性乗積 :math:`\int xy\,\mathrm{d}m` などに **負号を付けた値** である。
CAD ソフトの多くは慣性乗積を :math:`+\int xy\,\mathrm{d}m` の符号で表示するので、その値の符号を反転して設定する（:ref:`設定ファイルの [rocket] <sec-config-rocket>`）。
点質量 :math:`m` を :math:`(x,y,0)` に置くと :math:`I_{xy}=-mxy` になる。この符号は自動テストで確かめている（:doc:`verification`）。

乾燥時の慣性テンソルは、乾燥重心まわりの値を設定で与える。
推進剤を足した現在の慣性テンソルは、まず質量の中心を求め、各部分の慣性テンソルを平行軸の定理で現在の重心まわりに移して足す。

.. math::
   :label: eq-inertia-parallel

   I = I_d + m_d\left(|\bm{r}_d|^2E-\bm{r}_d\bm{r}_d^\top\right)
     + I_p + m_p\left(|\bm{r}_p|^2E-\bm{r}_p\bm{r}_p^\top\right)

:math:`E` は単位行列、:math:`\bm{r}_d,\bm{r}_p` は現在の重心から乾燥部分の重心と推進剤の重心までの機体座標のベクトルである。
推進剤は軸上にあるので、乾燥重心の横ずれを :math:`(y_d,z_d)` とすると、重心は :math:`y_{cg}=m_dy_d/m,\ z_{cg}=m_dz_d/m` にずれる。
推進剤の円柱の慣性テンソルは、:math:`I_p=\operatorname{diag}\!\left(\tfrac12m_pr_m^2,\ m_p(3r_m^2+L_m^2)/12,\ m_p(3r_m^2+L_m^2)/12\right)` である（:doc:`propulsion`）。

慣性乗積も重心の横ずれもなく :math:`I_{zz}=I_{yy}` のとき、慣性テンソルは :math:`\operatorname{diag}(I_{xx},I_{yy},I_{yy})` になり、
:eq:`eq-euler` を成分ごとの式として解く。それ以外では、逆行列を使って

.. math::
   :label: eq-euler-tensor

   \dot{\bm{\omega}} = I^{-1}\left(\bm{M}_b-\bm{\omega}\times(I\bm{\omega})\right)

とする。飛翔計算の前に、乾燥時と燃焼中（燃焼時間を 32 等分した時刻と推力曲線の点の時刻）の重心まわりの :math:`I` の固有値（主慣性モーメント）を求め、
すべてが有限で正で、最大と最小の比（条件数）が :math:`10^8` 以下であることを確かめる。満たさなければ、時刻と主慣性モーメントを示してエラーにする（:ref:`sec-config-checks`）。
逆行列を単位行列などで代えて計算を続けることはない。
質量の時間変化による :math:`\dot I` の項は無視する（:ref:`sec-lim-motion`）。

空力モーメント
~~~~~~~~~~~~~~

**軸対称表** では、法線力は圧力中心 :math:`x_{cp}` に働く :cite:`barrowman`。圧力中心の機体座標は :math:`(x_{cg}-x_{cp},0,0)` なので、復元モーメントは

.. math::
   :label: eq-moment-static

   \bm{M}_{N,b} = \begin{pmatrix}x_{cg}-x_{cp}\\0\\0\end{pmatrix}\times\bm{F}_{N,b}

である。:math:`x_{cp}>x_{cg}`\ （圧力中心が重心より後ろ）のとき、このモーメントは機首を対気速度の方向へ向ける。
これを **風見効果**\ （weathercocking）と呼ぶ。横風を受けると機体は風上を向くので、弾道落下の着地点は風上側にずれる。

これに\ :eq:`eq-damping` のピッチ減衰を加える。

.. math::
   :label: eq-moment-damp

   \bm{M}_{\mathrm{damp},b} = -\frac{qS}{V}\,\max\!\left(0,\ S_2-2x_{cg}S_1+x_{cg}^2S_0\right)\begin{pmatrix}0\\ \omega_y\\ \omega_z\end{pmatrix}

重心が軸から :math:`(y_{cg},z_{cg})` だけずれているときは、軸上に働く空気力 :math:`\bm{F}_{A,b}` が作るモーメント
:math:`(0,-y_{cg},-z_{cg})^\top\times\bm{F}_{A,b}` を加える。
ロールのモーメントは :ref:`sec-roll-moment` の :math:`q\,S\,d\,C_l^{\mathrm{tot}}` である。

**一般表** では、:eq:`eq-aero-moment6` が静的なモーメントとロールのモーメントを与える。
ピッチとヨーの減衰は、ピッチ用の和 :math:`S_0^q,S_1^q,S_2^q` とヨー用の和 :math:`S_0^r,S_1^r,S_2^r` を別々に持ち、

.. math::
   :label: eq-moment-damp6

   \bm{M}_{\mathrm{damp},b} = -\frac{qS}{V}\begin{pmatrix}0\\ K_q\,\omega_y\\ K_r\,\omega_z\end{pmatrix},\qquad
   K_q=\max\!\left(0,\ S_2^q-2x_{cg}S_1^q+x_{cg}^2S_0^q\right)

とする（:math:`K_r` は :math:`S^r` で同じ形）。部品積み上げ法とパネル法から展開した表では :math:`S^q=S^r` で、\ :eq:`eq-moment-damp` に一致する。
減衰は、重心が横にずれていても軸上の重心を仮定した式で求める（:ref:`sec-lim-motion`）。

.. _sec-aero-unsteady:

加速度と回転による空気力
~~~~~~~~~~~~~~~~~~~~~~~~

静的な係数と減衰のモーメント（:eq:`eq-moment-damp`）のほかに、飛翔計算は次の 4 つの項を加える。

* 見かけの質量の遅れ（:math:`\dot\alpha`）と対流の回転によるモーメント（表の ``lag_l0``、``lag_l1``）
* 回転による法線力（:math:`C_{Nq}`。減衰の和 :math:`S_0,S_1` から）
* Magnus 力（表の ``cnpa``）
* 誘導ロール（表の ``cl4``）

どれも状態量を増やさず、そのときの状態とその微分だけで決まる（準定常。:ref:`sec-quasi-steady`）。
表の列は :ref:`sec-file-axisym` に、各手法での値の出どころは :ref:`sec-aero-damping` にある。

見かけの質量の遅れと対流の回転
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

細長い物体の理論（slender-body theory）:cite:`munk,nielsenbook` では、機体の断面は、断面積 :math:`A(x)` に比例する空気の質量 :math:`\rho A(x)`\ （単位長さあたり）を横向きの運動に連れて動かす（見かけの質量）。
ノーズ先端から後方へ :math:`x`、重心からの距離を :math:`l=x-x_{cg}` とし、ピッチ面で書くと、断面の横向きの対気速度は :math:`w_c=u_z+\omega_y\,l` で（:math:`\omega_y>0` で機尾は :math:`+z_b` へ動く）、断面に働く力は

.. math::
   :label: eq-lag-section

   f(x)=-\rho\,\frac{D}{Dt}\bigl[A\,w_c\bigr],\qquad \frac{D}{Dt}=\frac{\partial}{\partial t}+V\frac{\partial}{\partial x},\qquad
   \frac{D}{Dt}\bigl[A\,w_c\bigr]=A\,(\dot u_z+l\,\dot\omega_y)+V A'\,(u_z+\omega_y\,l)+V A\,\omega_y

である（:math:`A'=\mathrm{d}A/\mathrm{d}x`）。右辺の各項の扱いは次のとおりである。

* :math:`V A' u_z`：静的な法線力（Munk の胴体の法線力）。係数表の :math:`C_N` に入っている。
* :math:`V A' \omega_y\,l`：回転による局所的な迎角の項。減衰の和 :math:`S_0,S_1,S_2`\ （:eq:`eq-damping`）に入っている。
* :math:`A\,\dot u_z`：横向きの対気速度の変化に遅れて付いてくる空気の反作用（:math:`\dot\alpha` の項、遅れの項）。この節で加える。
* :math:`V A\,\omega_y`：対流による回転の項。局所的な迎角の和は :math:`A'` の項だけを数えるので、この項は減衰の和に入っていない。この節で加える。
* :math:`A\,l\,\dot\omega_y`：角加速度による付加慣性。省く。

:math:`A\dot u_z` と :math:`VA\omega_y` の項は、どちらも断面積 :math:`A` に比例するので、機体全体では容積 :math:`\mathrm{Vol}` の中心 :math:`x_{\mathrm{vol}}` に働く力 :math:`-\rho\,\mathrm{Vol}\,(\dot u_z+V\omega_y)` になる。
係数表には、これを基準面積で無次元化した遅れの和

.. math::
   :label: eq-lag-sums

   L_0=\frac{2\,\mathrm{Vol}}{S}\ [\mathrm{m}],\qquad L_1=L_0\,x_{\mathrm{vol}}\ [\mathrm{m^2}]

を Mach 数ごとに持つ（``lag_l0``、``lag_l1``。部品積み上げ法での求め方は :ref:`sec-damping`）。
力 :math:`-\tfrac12\rho S L_0(\cdot)` が :math:`L_1/L_0` に働くとみなすと、重心まわりのモーメントは

.. math::
   :label: eq-lag-moment

   \Delta\bm{M}_{b}=-\frac{\rho S}{2}\,\bigl(x_{cg}L_0-L_1\bigr)\,f_{\mathrm{lag}}(\alpha_t')\;\bm{e}_x\times\bm{a}_\perp

である。:math:`\bm{e}_x=(1,0,0)`、:math:`\bm{a}_\perp=(0,a_y,a_z)` は

.. math::
   :label: eq-lag-accel

   \bm{a}=R^\top\bigl(\dot{\bm{v}}-\dot{\bm{W}}\bigr)-2\,\bm{\omega}_t\times\bm{u},\qquad \bm{\omega}_t=(0,\,\omega_y,\,\omega_z),\qquad
   \dot{\bm{W}}=\frac{\mathrm{d}\bm{W}}{\mathrm{d}h}\,\dot h

の横の成分である。:math:`\bm{u}` は機体座標の対気速度（:eq:`eq-alpha`）、:math:`\bm{W}` は風である。
:math:`R^\top(\dot{\bm{v}}-\dot{\bm{W}})-\bm{\omega}_t\times\bm{u}` が対気速度の変化（遅れの項の :math:`\dot u_z`）で、もう一つの :math:`-\bm{\omega}_t\times\bm{u}`\ （横の成分は :math:`u_x\approx V` で :math:`(0,-\omega_zV,\omega_yV)`）が対流の回転の項 :math:`V\omega_y` である。
対気速度の変化は、機体とともにロールしない軸で微分する（:math:`\bm{\omega}_t` はロール角速度を含まない）。軸対称な機体の見かけの質量は、ロールしても変わらないからである。
:math:`\dot{\bm{W}}` は、機体が風の高度分布の中を上下に動くことによる風の変化（風のシア）で、:math:`\mathrm{d}\bm{W}/\mathrm{d}h` を高さ :math:`\pm1` cm の中心差分で求める。
機体が回転しなくても、風が高さとともに変われば横向きの対気速度は変わり、このモーメントが働く。
:math:`\dot{\bm{v}}` はこの項を除いた力から求めた加速度で、モーメントだけを加え、力 :math:`-\rho\,\mathrm{Vol}\,(\dot u_z+V\omega_y)` は加えない（加えると加速度がこの力自身に依存し、代数的なループになる）。
見かけの質量の反作用は流れの向きによらないので、後ろ向きの飛行でも向きを反転しない。

:math:`f_{\mathrm{lag}}` は、迎角の大きい流れでこの項を消す係数である。

.. math::
   :label: eq-lag-fade

   f_{\mathrm{lag}}(\alpha)=|\cos\alpha|\,\bigl(1-3s^2+2s^3\bigr),\qquad s=\min\!\left(1,\ \max\!\left(0,\ \frac{\alpha-20^\circ}{10^\circ}\right)\right)

20° までは :math:`\cos\alpha`、20° から 30° で滑らかに 0 になり、30° を超えると 0 である。
見かけの質量の理論は小さな迎角の理論で、迎角が大きいと横流れがはく離し、静的な係数（:eq:`eq-body-lift`）がその力を表すからである。

飛行経路の向きがほぼ一定のピッチ振動では :math:`\dot u_z\approx V\omega_y` で、どちらの項も角速度と同じ位相で効くので、\ :eq:`eq-lag-moment` はピッチ減衰を :math:`K` から :math:`K-2\,(x_{cg}L_0-L_1)` に変える（:math:`K` は\ :eq:`eq-moment-damp` の括弧の中）。
重心が容積の中心より後ろ（:math:`x_{cg}>x_{\mathrm{vol}}`）なら減衰を弱め、前なら強める。
サンプル（:math:`M=0.3` で :math:`L_0=2.722` m、:math:`L_1=2.226` m\ :sup:`2`、:math:`x_{\mathrm{vol}}=0.818` m）では、:math:`2(x_{cg}L_0-L_1)/K` は燃焼前の重心 0.959 m で 0.25、燃焼後の重心 0.850 m で 0.048 である。

回転による法線力
^^^^^^^^^^^^^^^^

減衰のモーメントのもとになる局所的な迎角（:ref:`sec-damping`）は、モーメントだけでなく力も作る。
重心から :math:`l_i` 後ろの部品は、回転によって :math:`l_i\,(\bm{e}_x\times\bm{\omega})` の横速度で空気に対して動き、その法線力は静的な法線力と同じく、この横流れに逆らう向きに働く。
部品について足すと

.. math::
   :label: eq-cnq-force

   \Delta\bm{F}_{b}=-\frac{qS}{V}\,\bigl(S_1-x_{cg}S_0\bigr)\,\bm{e}_x\times\bm{\omega},\qquad \bm{e}_x\times\bm{\omega}=(0,\,-\omega_z,\,\omega_y)

である（Etkin の :math:`Z_q` にあたる :cite:`etkin`）。フィンが重心より後ろにあれば（:math:`S_1>x_{cg}S_0`）、:math:`\omega_y>0` で機尾は :math:`+z_b` 向きに動き、力は :math:`-z_b` を向く。
この力の重心まわりのモーメント :math:`\sum_i l_i\,\bm{e}_x\times\Delta\bm{F}_i=-(qS/V)\sum_iC_{N\alpha,i}l_i^2\,(0,\omega_y,\omega_z)` は、減衰のモーメント\ :eq:`eq-moment-damp` そのものである。
減衰のモーメントはすでに加えているので、この力は重心に働く力として加え、モーメントを二重に数えない。
一般表では、:math:`z_b` 方向の力にピッチの和 :math:`S_0^q,S_1^q`、:math:`y_b` 方向の力にヨーの和 :math:`S_0^r,S_1^r` を使う。

この力はピッチの固有振動数の 2 乗を :math:`1-c_q` 倍にする（:math:`c_q=\rho S(S_1-x_{cg}S_0)/(2m)`）。サンプルの地上付近では :math:`c_q` は 0.001〜0.002（:math:`M=0.3` の和、燃焼前と燃焼後）で、ほかの項より小さい。

Magnus 力
^^^^^^^^^

ロールしている機体が迎角を持つと、迎角の面に垂直な横力（Magnus 力）が生じる :cite:`mccoy`。

.. math::
   :label: eq-magnus

   \Delta\bm{F}_{b}=q\,S\,C_{Np\alpha}^{*}\,\hat p\,\frac{\bm{u}\times\bm{e}_x}{V},\qquad \hat p=\frac{\omega_xd}{2V},\qquad
   |\bm{u}\times\bm{e}_x|=V\sin\alpha_t

で、:math:`C_{Np\alpha}^{*}` は表の ``cnpa``\ [1/rad] である。
大きさは :math:`C_{N,\mathrm{Magnus}}=C_{Np\alpha}^{*}\,\hat p\,\sin\alpha_t` で、向き :math:`\bm{u}\times\bm{e}_x=(0,u_z,-u_y)` は静的な法線力の面から 90° 回った向きである。
:math:`\omega_x>0` で対気速度の横の成分が :math:`+z_b` 向き（:math:`u_z>0`）なら、``cnpa`` が正のとき力は :math:`+y_b` を向く。
McCoy の Magnus 力の係数 :math:`C_{Np\alpha}` は :math:`pd/V` で無次元化するので、:math:`\hat p=pd/(2V)` を使う表の値は :math:`C_{Np\alpha}^{*}=2C_{Np\alpha}` である。符号の約束（McCoy の :math:`\bm{V}\times\bm{x}` の向きを正）は同じである。
Magnus 力は静的な法線力の圧力中心 :math:`x_{cp}`\ （一般表では :eq:`eq-xcp-eff` の等価な圧力中心）に働くとし、そのモーメント :math:`(x_{cg}-x_{cp},0,0)^\top\times\Delta\bm{F}_b` を加える。
表は Magnus のモーメントの係数を別に持たないので、作用点を圧力中心とするのは仮定である。

誘導ロール
^^^^^^^^^^

十字形のフィンでは、横流れがフィンの面から外れた向きから当たると、迎角の 2 乗に比例するロールのモーメントが生じる。
軸対称表では、ロールの係数（:eq:`eq-cl-axisym`）に

.. math::
   :label: eq-induced-roll

   \Delta C_l=C_{l4}\,\alpha_t'^{\,2}\,\sin\bigl(4(\phi_c-\phi_f)\bigr)\,\sgn(u_x),\qquad \phi_c=\operatorname{atan2}(u_z,\,u_y)

を加える（:math:`C_{l4}` は表の ``cl4``、:math:`\alpha_t'` は rad）。
:math:`\phi_f` は付随ファイルの ``fin_roll_deg``\ （1 枚のフィンのロール角。:math:`+y_b` から :math:`+z_b` へ測る）である。
横流れがフィンの面にあるとき（:math:`+` 形と :math:`\times` 形）は対称なので 0 で、22.5° ずれたとき最大になる。カントの強制と同じく、後ろ向きの飛行では向きを反転する。
一般表では、展開のときにこの項を :math:`C_l` の列に入れる（:ref:`sec-aero-general`）。

.. _sec-roll-resonance:

ロールとピッチの共振の確認
^^^^^^^^^^^^^^^^^^^^^^^^^^

機体に固定した向きの釣り合いの強制（トリムの強制）があると、その強制は機体とともにロール角速度 :math:`p` で回る。
:math:`p` がピッチの固有振動数 :math:`\omega_n` に等しくなると、強制が固有振動数でピッチ・ヨーの振動を駆動し、迎角が大きくなる（ロール共振。ロール角速度がそこにとどまると、ロックインや破壊的な首振りになる）。
飛翔計算は、次のトリムの強制が 1 つでもあるとき、ロール角速度がピッチの固有振動数を横切った時刻を記録する。

* フィンのカント（``fin_cant_deg`` が 0 でない。モンテカルロ解析のばらつきを含む）
* 乾燥重心の横ずれ（``cg_lateral_dry`` が 0 でない）
* 推力線の傾きまたはノズルのオフセット
* 迎角 0、横滑り角 0 のモーメント :math:`|C_m|` または :math:`|C_n|` が :math:`10^{-4}` を超える一般表（非対称な機体の空力的なトリム）

ピッチの固有振動数は

.. math::
   :label: eq-roll-resonance

   \omega_n=\sqrt{\frac{\max(0,\,R)\,q\,S\,d}{\bar I}},\qquad \bar I=\frac{I_{yy}+I_{zz}}{2}

で、:math:`R` は静安定余裕の判定と同じ小迎角の復元の微係数（:ref:`sec-static-margin-general`。``cn_scale`` の倍率を含む）、:math:`\bar I` はそのときの横の慣性モーメントの平均である。
自由飛行のうち頂点より前で対気速度が 10 m/s を超える間の、受け入れたステップの始端ごとに :math:`|\omega_x|-\omega_n` を求め、符号が変わったステップで、最初の交差の時刻と Mach 数と振動数を線形補間で求める。
交差の回数も数える。結果は ``summary.json`` の ``roll_resonance``\ （:ref:`sec-file-summary`）に書き、警告を出す（:ref:`sec-config-checks`）。
トリムの強制がない機体（サンプルの既定の設定）では確かめない。

例えば、サンプルのフィンに 1° のカントを付けると、ロール角速度は上昇の初めにピッチの固有振動数を横切り、
``roll resonance: the roll rate crosses the pitch natural frequency (10.1 rad/s) at t = 0.61 s, Mach 0.36; fin cant, a lateral CG offset or thrust misalignment can amplify the angle of attack there``
の警告が出る。このあとロール角速度は固有振動数（最大で約 63 rad/s）より大きいまま約 88 rad/s まで上がり（:ref:`sec-vaero-barrowman-roll-flight`）、交差は 1 回である。
この確認は交差を知らせるだけで、迎角の増え方の大きさは飛翔計算そのもの（``trajectory.csv`` の ``alpha_deg``）で見る。

.. _sec-quasi-steady:

準定常の仮定
^^^^^^^^^^^^

この節の項は、どれもそのときの状態と加速度だけで決まり、流れの履歴を表す状態量（遅れの状態や動的失速の状態）を持たない。
その根拠は、機体の振動が流れの時間尺度に比べて遅いことである。
フィンの平均空力翼弦 :math:`c=0.155` m を使った減次振動数 :math:`k=\omega_nc/(2V)` は、サンプルの上昇中（対気速度 10 m/s 超）で 0.006〜0.010、中央値 0.007 である。
流れがフィンの翼弦を通り過ぎる時間 :math:`c/V` は、振動の周期の :math:`k/\pi\approx0.2\,\%` にあたる。
2 次元の翼の Theodorsen の関数 :math:`C(k)` :cite:`theodorsen` は、:math:`k=0.007` で :math:`0.988-0.035\,i` で、揚力の振幅を 1.1 % 下げ、位相を 2.0°（周期の 0.6 %）遅らせる。
アスペクト比の小さいフィンでは、後流の影響はこれより小さい。流れの履歴による揚力の大きさの変化は 1 % 程度で、法線力の不確かさ（``cn_scale``）よりずっと小さい。
ただし、遅れの位相の成分は、2 次元の翼ではフィンの準定常のピッチ減衰（局所的な迎角の振幅は迎角の振幅の :math:`\omega_n l_f/V=0.033`〜0.065 倍、:math:`l_f` はフィンの重心からの距離）と同じ程度の大きさになりうる。
減衰の不確かさは ``cmq_scale``\ （:ref:`sec-mc`）で見込む。
動的失速や大迎角でのはく離の遅れは、この見積もりの外で、扱わない（:ref:`sec-lim-motion`）。

.. _sec-roll-moment:

ロールのモーメント
~~~~~~~~~~~~~~~~~~

ロールのモーメントは、フィンのカントによる強制項と、ロール角速度による減衰項からなる。
ロール角速度 :math:`\omega_x` を無次元化して :math:`\hat p=\omega_xd/(2V)` と書くと、ロールの係数は

.. math::
   :label: eq-cl-total

   C_l^{\mathrm{tot}} = C_l + C_{l\delta}\,(\delta-\delta_{\mathrm{ref}})\,\sgn(u_x) + C_{lp}\,\hat p

である（:math:`C_{l\delta}` はフィンのカント :math:`\delta`\ [rad] あたりのロールモーメント係数、:math:`C_{lp}<0` はロール減衰微係数）。
:math:`\delta` は設定したカント（モンテカルロ解析ではばらつきを加えたもの）、:math:`\delta_{\mathrm{ref}}` は表の :math:`C_l` に **すでに含まれている** カントである。
差だけを足すので、表を作ったカントで飛ぶときに強制項を二重に数えることはない。
後ろ向きの飛行（:math:`u_x<0`）ではフィンが流れを後ろから受けるので、カントによるロールのモーメントの向きが全体として反転する。
表に含まれる :math:`\delta_{\mathrm{ref}}` の分だけでなく、カントの差 :math:`\delta-\delta_{\mathrm{ref}}`\ （モンテカルロ解析のカントのばらつきを含む）の項も、基準のカントと同じ向きで反転する。
:eq:`eq-cl-total` は一般表の式である。軸対称表は :math:`C_l` の列を持たないので、

.. math::
   :label: eq-cl-axisym

   C_l^{\mathrm{tot}} = C_{l\delta}\,\delta\,\sgn(u_x) + C_{lp}\,\hat p

とする。第 1 項がカントの強制項（表に含まれる基準の分とばらつきの分の和）で、後ろ向きの飛行では向きが反転する。表の ``cl4`` が 0 でなければ、誘導ロール（:eq:`eq-induced-roll`）を加える。
軸対称表から展開した一般表の :math:`C_l` も、同じ :math:`C_{l\delta}\,\delta_{\mathrm{ref}}\,\sgn(u_x)` である。

部品積み上げ法とパネル法は、設定したカントで表を作る（:math:`\delta_{\mathrm{ref}}=\delta`）。
このとき強制項は :math:`C_{l\delta}\,\delta` で、モンテカルロ解析のカントのばらつきだけが差の項になる。
:math:`\delta=\delta_{\mathrm{ref}}` で、ロールの慣性が小さく角速度が準定常のとき、:math:`C_l^{\mathrm{tot}}=0` から定常なロール角速度が

.. math::
   :label: eq-roll-steady

   \omega_x = -\frac{2V\,C_{l\delta}\,\delta}{C_{lp}\,d}

と求まる（軸対称表、またはそれを展開した一般表の場合）。:math:`C_{l\delta}` と :math:`C_{lp}` は部品積み上げ法で求める（:ref:`sec-barrowman-roll`）。
ロールとピッチ・ヨーは、ロール角速度に比例する Magnus 力（:eq:`eq-magnus`）と、迎角の 2 乗に比例する誘導ロール（:eq:`eq-induced-roll`）を通じてつながり、どちらも表の値（既定では 0）で決まる。
軸対称な機体の線形の交差微係数 :math:`C_{mp},C_{np},C_{mr},C_{nq},C_{lq},C_{lr}` は対称性から 0 なので、扱わない（:ref:`sec-lim-motion`）。

.. _sec-thrust-line:

推力線
~~~~~~

推力は、機軸に対して角度 :math:`\varepsilon`\ （``misalignment_deg``）だけ傾いた直線上に、ノズル出口の点 :math:`\bm{r}_n` を通って働く。
傾きの向きは方位角 :math:`\phi`\ （``misalignment_azimuth_deg``）で表し、:math:`+y_b` から :math:`+z_b` へ測る。
推力の方向とノズル出口の機体座標は

.. math::
   :label: eq-thrust-dir

   \bm{e}_T=\begin{pmatrix}\cos\varepsilon\\ \sin\varepsilon\cos\phi\\ \sin\varepsilon\sin\phi\end{pmatrix},\qquad
   \bm{r}_n=\begin{pmatrix}-x_{\mathrm{aft}}\\ o_y\\ o_z\end{pmatrix}

で、:math:`(o_y,o_z)` は設定の ``misalignment_offset``、:math:`x_{\mathrm{aft}}` はモータ後端の位置である。
推力の大きさは :math:`T` のままで、推力と、重心まわりのモーメントは

.. math::
   :label: eq-thrust-force

   \bm{F}_{T,b}=T\,\bm{e}_T,\qquad
   \bm{M}_{T,b}=(\bm{r}_n-\bm{r}_{cg})\times\bm{F}_{T,b},\qquad
   \bm{r}_{cg}=\begin{pmatrix}-x_{cg}\\ y_{cg}\\ z_{cg}\end{pmatrix}

である。ミスアライメントとノズルのオフセットがなく重心が軸上にあるとき、:math:`\bm{M}_{T,b}=0` である。
ノズルは重心より後ろにあるので、:math:`+z_b` 向きの横推力は :math:`+y_b` まわりのモーメントを作り、機首を :math:`-z_b` 側へ向ける。
角度の定義は :numref:`fig-body-axes` に示す。

ランチャ上では、推力の軸方向成分 :math:`T\cos\varepsilon` だけが機体をレールに沿って押す（:eq:`eq-rail`）。

飛翔の段階
----------

飛翔を :numref:`fig-phases` の段階に分け、段階ごとに異なる運動方程式を使う。

.. _fig-phases:

.. figure:: _generated/tikz/phases.*
   :width: 100%

   飛翔の段階と、段階が切り替わる条件。パラシュート降下でも、開傘の前に着地すれば自由飛行から着地する。二段開傘では、``main_altitude`` 以下でメイン降下に切り替わる。

.. _sec-rail:

ランチャ滑走
~~~~~~~~~~~~

ランチャ上では、機体はレールの方向 :math:`\bm{d}`\ （:eq:`eq-rail-dir`）にしか動けず、姿勢も変わらない。
レールに沿った速さを :math:`v_\parallel=\bm{v}\cdot\bm{d}` として

.. math::
   :label: eq-rail

   \dot{\bm{v}} = a_\parallel\,\bm{d},\qquad
   a_\parallel = \frac{T\cos\varepsilon - C_A\,q\,S\,\sgn(v_\parallel)}{m} + \bm{g}\cdot\bm{d}

とする。:math:`\varepsilon` は推力のミスアライメント角（:ref:`sec-thrust-line`）で、:math:`C_A` は迎角 0 の値（一般表では :math:`-C_X`）である。
機体はレールに拘束されているので、推力の横成分とそのモーメントは姿勢に影響しない。
初期姿勢はレールの方向と ``launch.roll_deg`` で決まり（:ref:`sec-initial-attitude`）、ランチャ上ではこの姿勢のままである。
:math:`v_\parallel\le0` かつ :math:`a_\parallel<0` のとき（推力が重力に負けて静止しているとき）は :math:`a_\parallel=0` とする。
減速するステップでは、この規則が働く前に、ステップの終端の :math:`v_\parallel` がわずかに負になることがある。
そのときは機体を後ろへ動かさず、ステップの両端の :math:`v_\parallel` を線形に結んだ速さが 0 になる時刻を求め、その時刻の三次エルミート補間（:eq:`eq-step-hermite`）の位置で、速度 0 で止める。
止める位置は、そのステップの始端の位置とレールの基部より後ろにはしない。
レールに沿った移動距離が :math:`\bm{x}\cdot\bm{d}\ge L_{\mathrm{rail}}` になったステップでは、そのステップの両端の位置と速度から作る三次エルミート補間（:eq:`eq-step-hermite`）の上で
:math:`\bm{x}\cdot\bm{d}=L_{\mathrm{rail}}` となる時刻を求める。離脱の状態は、位置がこの補間、速度がその時間微分、姿勢などの残りが両端の線形補間である（位置が時間の三次式で表される運動、たとえば加速度が時間に線形な運動では、補間は厳密である）。
この時刻にランチャを離れ、自由飛行に移る。積分は、この離脱の時刻と状態から自由飛行の運動方程式で **やり直す**\ （レールを越えたステップの終端の状態は使わない）。
離脱の状態の速さを **ランチャ離脱速度**、静安定余裕を離脱時の値として記録する。
燃焼が終わったあとも飛翔が続く間は、機体はレール上を進み続ける（慣性で残りの距離を進んで離れうる）。
燃焼終了後に機体のレールに沿った速度が 0 以下になった、つまりレールを離れずに滑り落ち始めた場合に限り、推力不足としてエラー（``rocket never left the launch rail``）にする。

自由飛行
~~~~~~~~

:eq:`eq-translation` と :eq:`eq-euler` による 6 自由度の運動である。
弾道落下モードでは着地までこのまま計算する。

頂点のあとの弾道落下では、全迎角が 60° から 120° の間（流れに対して横向き）で飛んだ時間を ``summary.json`` の ``broadside_time`` [s] に積算する（ステップの始端の迎角で判定し、着地のステップは着地までの分だけ数える。パラシュート降下では 0）。
これが 10 s を超えた落下を「横向きに落ちた」とし、``fell_broadside`` を ``true`` にして、
``the ballistic descent fell broadside (angle of attack 60 to 120 deg for 146 s): the landing depends on the large-angle aerodynamics, which are not validated`` の形（数は例）の警告を出す。
横向きの落下は大迎角でのつり合い（トリム）で、その着地点は検証していない大迎角の空力係数で決まるからである（:ref:`sec-lim-aero`）。結果は使えないものとはせず、終了ステータスは変えない。

.. _sec-parachute:

パラシュート降下
~~~~~~~~~~~~~~~~

頂点（:math:`v_z` の符号が正から負に変わった時刻 :math:`t_{\mathrm{apo}}`。補間で求める。:ref:`sec-integrators`）から遅れ :math:`\Delta t` の後に開傘する。
開傘の時刻は、刻み幅によらず正確に :math:`t_{\mathrm{apo}}+\Delta t` である。刻みが開傘の瞬間を越えたときは、その瞬間まで状態を補間（位置は 3 次 Hermite 補間、速度はその時間微分）で戻して開傘する。
開傘した時刻は ``summary.json`` の ``drogue_deploy_time`` に記録する。着地までに開傘しなかった（頂点 + 遅れより前に着地した）飛翔と、弾道落下では出力しない。

:math:`t_{\mathrm{apo}}+\Delta t` にモータがまだ燃えているとき（推力の終わりの時刻 :math:`t_{\mathrm{end}}` より前のとき）は、開傘を :math:`t_{\mathrm{end}}` まで遅らせる。
:math:`t_{\mathrm{end}}` は、推力曲線のうち推力が 0 でない最後の区間の終わりの点の時刻である（推力が 0 のまま続く末尾の点は数えない）。
パラシュート降下の運動方程式（:eq:`eq-parachute`）は推力を持たず、推進剤の消費も扱わないので、推力のある間に開傘すると、残りの推力と推進剤が消えてしまうからである。
遅らせた開傘は ``summary.json`` の ``deploy_delayed`` に、本来の開傘時刻 ``nominal_time``\ （:math:`t_{\mathrm{apo}}+\Delta t`）とその時刻の推力 ``thrust`` [N] として残し、
``the motor still burns at apogee + recovery.delay (t = 11.25 s, thrust 42.0 N): the parachute deployment is delayed to burnout at 12.50 s`` の形（数は例。二段開傘では ``the drogue deployment``）の警告を出す。
``drogue_deploy_time`` は遅らせた時刻 :math:`t_{\mathrm{end}}` である。燃焼時間が頂点より長い、推力の小さい長い燃焼のモータで起こる。
開傘後は機体をパラシュートに吊られた質点とみなし、

.. math::
   :label: eq-parachute

   \dot{\bm{v}} = -\frac{\rho\,|\bm{v}_a|\,(C_DS)_p}{2m}\,\bm{v}_a + \bm{g},\qquad \bm{v}_a=\bm{v}-\bm{w}

とする :cite:`knacke`\ （:math:`(C_DS)_p` は設定 ``cd_s``）。終端降下速度は :math:`v_t=\sqrt{2mg/(\rho\,(C_DS)_p)}` で、
サンプル機体（燃焼後 8 kg、:math:`(C_DS)_p=3.5\ \mathrm{m^2}`）では海面付近で約 6.0 m/s になる。
開傘の衝撃や、開傘までの過渡的な挙動は扱わない。開傘は瞬時に完了し、抗力面積は開傘の瞬間に :math:`0` から :math:`(C_DS)_p` へ跳ぶ（:doc:`limitations`）。

.. _sec-dual-deploy:

二段開傘
^^^^^^^^

設定 ``main_cd_s`` と ``main_altitude`` を与えると、二段開傘（ドローグとメイン）になる。
``cd_s`` と ``delay`` のパラシュートがドローグで、頂点から ``delay`` 後に開傘する。
下降して射点からの高さ :math:`h` が ``main_altitude`` :math:`h_m`\ （射点からの高さ、AGL）に達すると、メインが開傘し、
ドローグの抗力面積をメインの抗力面積 :math:`(C_DS)_m`\ （``main_cd_s``）に **置き換える**\ （足し合わせない。ドローグはメインの後ろで切り離されるか、つぶれるとみなす）。

.. math::
   :label: eq-dual-deploy

   (C_DS)(h)=\begin{cases}(C_DS)_p & h>h_m\\ (C_DS)_m & h\le h_m\end{cases}

運動方程式は\ :eq:`eq-parachute` のままで、:math:`(C_DS)_p` を上の値に替える。
メインの開傘もドローグと同じく瞬時で、開傘の衝撃は扱わない。
高さが :math:`h_m` を横切るステップでは、そのステップの端点から横切る時刻まで戻り、位置は三次エルミート補間、速度はその時間微分で求めて、そこからメインの抗力面積で続ける。
横切る時刻と高さ :math:`h_m` は ``summary.json`` の ``main_deploy_time`` と ``main_deploy_altitude`` に書く。
ドローグの開傘がすでに :math:`h_m` 以下で起きるとき（頂点が低いときなど）は、ドローグの開傘と同時にメインが開き、その時刻と高さを記録する。
ドローグとメインの終端降下速度は、それぞれ :math:`\sqrt{2mg/(\rho\,(C_DS)_p)}` と :math:`\sqrt{2mg/(\rho\,(C_DS)_m)}` である。
弾道落下ではメインを使わず、``main_deploy_time`` と ``main_deploy_altitude`` は書かない。
モンテカルロ解析の ``parachute_cd_s_scale`` は、``cd_s`` と ``main_cd_s`` の両方に同じ倍率を掛ける。

.. _sec-landing:

着地
~~~~

ランチャ離脱後に、ステップの始端で :math:`z_U>0`、終端で :math:`z_U\le0` となったステップで計算を終える（ECEF モードでは測地的な高さ :math:`h-h_0` について同じ）。
着地には、この高さの区間（始端で正、終端で 0 以下）が必要である。そのステップの三次エルミート補間（:eq:`eq-step-hermite`）の高さの零点を二分法で求めて着地時刻とし、
その時刻の位置（エルミート補間）と速度（エルミート補間の時間微分）を着地点・着地速度とする。``trajectory.csv`` の最終行もこの着地の状態である。着地点は\ :eq:`eq-enu-basis` と :eq:`eq-ecef-lla` で緯度・経度に変換する。

.. _sec-integrators:

数値積分
--------

状態方程式 :math:`\dot{\bm{y}}=\bm{f}(t,\bm{y})` を、設定 ``sim.integrator`` で選んだ方法で積分する。
どの方法でも、1 ステップの後で（``attitude = "normalize"`` ではクォータニオンを正規化したうえで）、段階の切り替え（ランチャ離脱・頂点・開傘・着地）を判定する。
ステップの途中で起きる事象（ランチャ離脱・頂点・開傘・メインの開傘・着地）は、そのステップ内の補間で時刻と状態を求めて、次の項のとおりに扱う。
状態量に有限でない値が現れたときは、``integration diverged at t = … s; reduce sim.dt or tighten rtol/atol`` というエラーで止める（終了コード 1。モンテカルロ法ではそのサンプルを失敗として数え、風の格子ではそのケースを失敗として記録する）。
値が有限でも、速さが 8 km/s を超えるか高さが 1000 km を超えた状態は、ロケットが到達しない数値的な発散とみなし、``(speed … m/s is not physical)`` または ``(altitude … m is not physical)`` を添えた同じ形のエラーで止める。

.. _sec-step-events:

ステップ内の事象と刻みの区切り
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

刻み幅 :math:`h` のステップの両端の位置 :math:`\bm{x}_0,\bm{x}_1` と速度 :math:`\bm{v}_0,\bm{v}_1` から、位置の三次エルミート補間

.. math::
   :label: eq-step-hermite

   \bm{x}(s)=(2s^3-3s^2+1)\,\bm{x}_0+(s^3-2s^2+s)\,h\bm{v}_0+(-2s^3+3s^2)\,\bm{x}_1+(s^3-s^2)\,h\bm{v}_1,\qquad s\in[0,1]

を作る。ランチャ離脱、メインの開傘、着地は、この補間した位置から求めた量（レールに沿った距離、高さ）の零点を、
:math:`s` の区間 :math:`[0,1]` で二分法（区間の幅がステップの :math:`2^{-52}` 倍になるまで）で求める。
頂点は、この補間の鉛直速度（:eq:`eq-step-hermite` の時間微分の鉛直成分）の零点を、同じ二分法で求める（後述の「頂点の時刻」）。ランチャ離脱、開傘、メインの開傘、着地の状態は、位置をこの補間、速度をその時間微分

.. math::
   :label: eq-step-hermite-vel

   \bm{v}(s)=\frac{1}{h}\frac{\dd\bm{x}}{\dd s}=\frac{6(s^2-s)}{h}\,(\bm{x}_0-\bm{x}_1)+(3s^2-4s+1)\,\bm{v}_0+(3s^2-2s)\,\bm{v}_1

で作り、姿勢と角速度などの残りは両端の線形補間で作る。

* **ランチャ離脱**：レールの終端を越えたステップで、その時刻まで戻り、そこから自由飛行として積分をやり直す（:ref:`sec-rail`）。
* **開傘とメインの開傘**：開傘の時刻は頂点の時刻 + ``delay`` ちょうど（モータがまだ燃えていれば推力の終わり。:ref:`sec-parachute`）、メインの開傘は高さが ``main_altitude`` を横切る時刻である。その時刻まで戻り、新しい抗力面積でそこから続ける。
* **着地**：高さの区間（始端で正、終端で 0 以下）がそろったステップで起こる（:ref:`sec-landing`）。

1 つのステップに複数の事象が入るときは、時刻の早い順に扱う。着地は、同時刻の事象に優先し、着地より後の事象は起こらない
（開傘の時刻が着地より後ろにあるステップでは、開傘しない）。

どの積分法でも、刻みは次の時刻をまたがない。ステップの終端が、これらのうち最も早い時刻を越えるときは、刻みを切り詰めてちょうどその時刻にそろえる。

* ``sim.max_time``：着地していなくても、この時刻に計算を終える（着地なしの結果。:ref:`sec-exit-status`）。
* 次の推力曲線の折れ点（燃焼終了を含む）。1 ステップは推力曲線の 1 つの区間の中に収まり、推力はその区間の線形な値を刻みの全体で使う。折れ点の時刻の値は、直前の区間の側の極限である。
  パワーオンとパワーオフの軸力係数は、燃焼終了を終端とするステップの中では切り替わらない。
* パラシュート降下で開傘前なら、開傘の時刻。

推力曲線の区間が変わったあとは、前のステップから持ち越した右辺（後述の FSAL）を使わずに、右辺を評価し直す。

古典的 Runge–Kutta 法（``rk4``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

固定刻み :math:`\Delta t`\ （``sim.dt``）の古典的 4 次 Runge–Kutta 法である :cite:`hairer`。

.. math::
   :label: eq-rk4

   \begin{aligned}
   \bm{k}_1 &= \bm{f}(t,\ \bm{y}_n), &
   \bm{k}_2 &= \bm{f}\!\left(t+\tfrac{\Delta t}{2},\ \bm{y}_n+\tfrac{\Delta t}{2}\bm{k}_1\right),\\
   \bm{k}_3 &= \bm{f}\!\left(t+\tfrac{\Delta t}{2},\ \bm{y}_n+\tfrac{\Delta t}{2}\bm{k}_2\right), &
   \bm{k}_4 &= \bm{f}\!\left(t+\Delta t,\ \bm{y}_n+\Delta t\,\bm{k}_3\right),\\
   \bm{y}_{n+1} &= \bm{y}_n + \frac{\Delta t}{6}\left(\bm{k}_1+2\bm{k}_2+2\bm{k}_3+\bm{k}_4\right). & &
   \end{aligned}

ランチャ離脱・頂点・着地は、どの積分法でもステップ内の補間で求めるので、刻み幅に依らない（以下の「頂点の時刻」）。
パラシュートの開傘は、頂点の時刻に ``delay`` を足した時刻ちょうどに行う。頂点を過ぎたあとの刻みは、その時刻にステップの端が合うように短くする。
``delay`` が刻みより短いときは、頂点を含むステップの終端が開傘の時刻を越えてしまうので、開傘の時刻まで状態を戻してから開く（位置は 3 次 Hermite 補間、速度はその時間微分）。
どの積分法でも、どの刻み幅でも、開傘の時刻は頂点の時刻 + ``delay``\ （推力の終わりまで遅らせたときはその時刻。:ref:`sec-parachute`）に一致する。
頂点の時刻は、次に述べる補間で求める。

推力曲線の折れ点や燃焼終了では、右辺 :math:`\bm{f}` が時間について滑らかでなくなる。刻みはこれらの時刻にちょうど合わせて切り詰めるので（:ref:`sec-step-events`）、
右辺の 4 回の評価は折れ点をまたがず、1 ステップの推力は 1 つの線形な区間の値だけを使う。したがって最後のステップは :math:`\Delta t` より短くなりうる。
1 ステップで右辺を 4 回評価する。

Dormand–Prince 法（``rk45``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

刻み幅を誤差に応じて自動で調整する、埋め込み型の 5(4) 次 Runge–Kutta 法（Dormand–Prince 法 :cite:`dormand`）である :cite:`hairer`。
刻み幅 :math:`h` の 1 ステップは 7 段で、

.. math::
   :label: eq-dopri

   \bm{k}_i&=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\quad (i=2,\dots,6),\\
   \bm{y}_{n+1}&=\bm{y}_n+h\sum_{j=1}^{6}b_j\bm{k}_j,\qquad
   \bm{k}_7=\bm{f}(t+h,\ \bm{y}_{n+1})

とする。係数は\ :eq:`eq-butcher` の Butcher 表 :cite:`dormand` のとおりで、5 次の解の重み :math:`b_j` は第 7 段の行 :math:`a_{7j}`\ （:math:`c_7=1`）と同じで、:math:`b_7=0` である。第 7 段 :math:`\bm{k}_7` は終点 :math:`(t+h,\ \bm{y}_{n+1})` の傾きである。

.. math::
   :label: eq-butcher

   \begin{array}{c|cccccc}
   0 & & & & & & \\
   \frac15 & \frac15 & & & & & \\
   \frac3{10} & \frac3{40} & \frac9{40} & & & & \\
   \frac45 & \frac{44}{45} & -\frac{56}{15} & \frac{32}{9} & & & \\
   \frac89 & \frac{19372}{6561} & -\frac{25360}{2187} & \frac{64448}{6561} & -\frac{212}{729} & & \\
   1 & \frac{9017}{3168} & -\frac{355}{33} & \frac{46732}{5247} & \frac{49}{176} & -\frac{5103}{18656} & \\
   \hline
   b_j & \frac{35}{384} & 0 & \frac{500}{1113} & \frac{125}{192} & -\frac{2187}{6784} & \frac{11}{84}
   \end{array}

.. math::
   :label: eq-butcher-e

   (E_1,\dots,E_7)=\left(\frac{71}{57600},\ 0,\ -\frac{71}{16695},\ \frac{71}{1920},\ -\frac{17253}{339200},\ \frac{22}{525},\ -\frac1{40}\right)

:eq:`eq-butcher-e` の :math:`E_j` は 5 次の解と 4 次の解の差を与える係数で、誤差の推定に使う。
:math:`\bm{k}_7` は新しい点での右辺そのものなので、受理されたステップの :math:`\bm{k}_7` を次のステップの :math:`\bm{k}_1` として再利用する
（FSAL: first same as last :cite:`hairer`）。したがって 1 ステップあたりの右辺の評価は 6 回である。
段階が切り替わったステップの後と、推力曲線の区間が変わったステップの後は右辺が変わるので、:math:`\bm{k}_1` を評価し直す。
5 次の解 :math:`\bm{y}_{n+1}` のクォータニオンは、:math:`\bm{k}_7` を評価する前に正規化する。

誤差と刻み幅の制御
^^^^^^^^^^^^^^^^^^

誤差の推定は :math:`\bm{e}=h\sum_{j=1}^{7}E_j\bm{k}_j` で、13 個の状態量すべて（位置・速度・クォータニオン・角速度。単位や大きさの重み付けはしない）について
相対許容誤差 ``rtol``\ （:math:`\varepsilon_r`）と絶対許容誤差 ``atol``\ （:math:`\varepsilon_a`）で規格化し、二乗平均平方根をとる :cite:`hairer`。
なお、刻み幅は許容誤差のほかに ``sim.output_interval`` でも上限が抑えられる（燃焼中とレール上は 0.05 s 以下）。このため、通常の飛翔では ``rtol``、``atol`` を小さくしても刻み幅が変わらず、結果がほとんど変わらないことがある。

.. math::
   :label: eq-dopri-err

   \mathrm{err}=\sqrt{\frac1{13}\sum_{c=1}^{13}\left(\frac{e_c}{\varepsilon_a+\varepsilon_r\max(|y_{n,c}|,\,|y_{n+1,c}|)}\right)^2}

:math:`\mathrm{err}\le1` ならステップを受理し、そうでなければ刻み幅を小さくして同じステップをやり直す。
次の刻み幅（やり直す場合はそのときの刻み幅）には、

.. math::
   :label: eq-step-factor

   h\leftarrow h\cdot f,\qquad f=\min\!\left(5,\ \max\!\left(0.2,\ 0.9\,\mathrm{err}^{-1/5}\right)\right)

を掛ける :cite:`hairer`。安全率は 0.9、倍率の範囲は 0.2〜5 で、:math:`\mathrm{err}=0` のときは 5、:math:`\mathrm{err}` が有限でないときは 0.2 とする
（:numref:`fig-step-factor`）。やり直しで刻み幅が :math:`10^{-6}` s を下回ると、
「刻み幅が下限を下回った」というエラーで終了する（許容誤差を緩めるか ``rk4`` を使う）。
最初の刻み幅は ``sim.dt`` である。

.. _fig-step-factor:

.. figure:: _generated/plots/step_factor.*
   :width: 75%

   規格化誤差と刻み幅の倍率。:math:`\mathrm{err}<0.9^5\approx0.59` なら広げ、:math:`\mathrm{err}>1` なら縮めてやり直す。
   倍率は :math:`\mathrm{err}<1.9\times10^{-4}` で上限の 5、:math:`\mathrm{err}>1.8\times10^{3}` で下限の 0.2 になる。

刻み幅の上限と調整
^^^^^^^^^^^^^^^^^^

各ステップの刻み幅は、制御された刻み幅と次の上限のうち小さい方である。

* 出力間隔 ``output_interval``\ （既定 0.05 s）。
* ランチャ上と燃焼中（:math:`t<t_b`）は 0.05 s。
* ランチャ上では、レールの終端に **ちょうど** 届くように合わせる。レールに沿った現在の速さ :math:`v_\parallel`、加速度 :math:`a_\parallel`、
  残りの距離 :math:`s_r=L_{\mathrm{rail}}-\bm{x}\cdot\bm{d}` から、加速度が一定として
  :math:`h\le\bigl(-v_\parallel+\sqrt{v_\parallel^2+2a_\parallel s_r}\bigr)/a_\parallel+10^{-6}` s とする（:math:`a_\parallel>0` かつ :math:`s_r>0` のとき）。
  これでランチャ離脱の時刻と速度が刻み幅ではなく :math:`10^{-6}` s 程度の精度で求まる。
* 次の推力曲線の折れ点（燃焼終了を含む）までの時間（折れ点をまたがない）。
* ``sim.max_time`` までの時間。
* パラシュート降下で開傘前なら、開傘時刻までの時間（開傘の瞬間にステップの端を合わせる）。

最後の 3 つの上限は、``rk4`` の固定刻みにも同じように働く（:ref:`sec-step-events`）。

上限で刻み幅を切り詰めたステップは、制御された刻み幅を小さくしない。
切り詰めたステップが受理されたときの次の刻み幅は :math:`\max(h_{\mathrm{ctrl}},\,h f)` で、
切り詰めのないとき（やり直しを含む）は :math:`h f` である。

頂点の時刻は、どの積分法でも、:math:`v_z` が正から 0 以下に変わったステップの中で、そのステップの三次エルミート補間（:eq:`eq-step-hermite`）の鉛直速度

.. math::
   :label: eq-apogee-vz

   v_z(s)=\frac{1}{h}\,\frac{\mathrm{d}\bm{x}}{\mathrm{d}s}\cdot\bm{e}_U\bigl(\bm{x}(s)\bigr)

が 0 になる点 :math:`s_a` を二分法で求め、:math:`t_{\mathrm{apo}}=t-h+h\,s_a` とする（:math:`\bm{e}_U` はその位置での上向きの単位ベクトル。平面地球では :math:`z_U` の向き）。
頂点の高さと水平位置は、同じ補間を :math:`s_a` で評価した値で、``apogee``、``apogee_east``、``apogee_north`` に記録する。
時刻と位置は同じ補間の上の点なので、補間した高さの極大になる。
開傘はこの時刻に ``delay`` を足した時刻に行う（:ref:`sec-parachute`）。着地の位置と時刻は、三次エルミート補間した高さの零点で求める（:ref:`sec-landing`）。
``trajectory.csv`` には、時刻が ``output_interval`` の倍数以上になった最初のステップの端点を書き出す（補間はしないので、間隔は完全には一定でない）。

``rk4`` と ``rk45`` の比較
^^^^^^^^^^^^^^^^^^^^^^^^^^

サンプルで比較した結果を :numref:`tbl-integrators` に示す。「降下」はパラシュート降下を含む約 685 s の飛翔、「弾道」は弾道落下の飛翔（約 62 s）の右辺の評価回数で、着地点と頂点は前者の値、時間は前者を単一スレッドで計算した実測値である。
``rk4`` の :math:`\Delta t=0.5` ms を参照解とみなすと、:math:`\Delta t=2` ms の ``rk4`` と既定の許容誤差（``rtol=1e-7``, ``atol=1e-6``）の ``rk45`` は、
頂点高度で 0.002 m 以内、着地点で 0.011 m 以内で一致する。右辺の評価回数は ``rk45`` が約 1/16 で済む。

.. _tbl-integrators:

.. list-table:: 積分法の比較（サンプル）
   :header-rows: 1
   :widths: 24 16 14 12 14 14 12

   * - 積分法
     - 評価回数（降下）
     - 評価回数（弾道）
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
     - 時間 [s]
   * - ``rk4``, :math:`\Delta t=0.5` ms（参照）
     - 5479096
     - 494976
     - 4439.94
     - −655.30
     - −7702.05
     - 0.69
   * - ``rk4``, :math:`\Delta t=2` ms（既定）
     - 1369780
     - 123748
     - 4439.95
     - −655.30
     - −7702.05
     - 0.18
   * - ``rk4``, :math:`\Delta t=10` ms
     - 273964
     - 24756
     - 4439.98
     - −655.30
     - −7702.10
     - 0.04
   * - ``rk45``, 既定の許容誤差
     - 83116
     - 8397
     - 4439.95
     - −655.30
     - −7702.06
     - 0.018

``rk45`` の評価回数のほとんどは、出力間隔による上限（0.05 s）で決まる。降下中は解が滑らかなので誤差の条件より先に上限に達し、
:math:`685\ \mathrm{s}/0.05\ \mathrm{s}\times6\approx82\,000` 回がその大半を占める。
許容誤差を 100 倍に緩めても評価回数は 82 420 回とほとんど変わらないのはこのためである
（``output_interval`` を大きくすれば、さらに減らせる）。
一方、許容誤差を極端に小さくすると（例えば ``rtol = atol = 1e-9`` で弾道落下の着地直前）、刻み幅が下限を下回ってエラーになることがある。
ランチャ離脱の時刻と速度は、どの積分法でも、レールの終端を越えたステップの中の補間で求めるので、刻み幅にほとんど依存しない。
``rk4`` は :math:`\Delta t=2` ms で 0.24715 s、45.661 m/s、:math:`\Delta t=10` ms で 0.24712 s、45.660 m/s、
参照解は 0.24715 s、45.661 m/s で、``rk45`` と ``dop853`` はともに 0.24715 s、45.661 m/s である。

Dormand–Prince 8(5,3) 法（``dop853``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Prince と Dormand の 8 次の埋め込み公式 :cite:`princedormand` を、Hairer と Wanner の実装（``dop853.f``）に従って用いる :cite:`hairer`。
刻み幅 :math:`h` の 1 ステップは 12 段で、8 次の解を伝播させる。

.. math::
   :label: eq-dop853

   \bm{k}_i=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\ (i=1,\dots,12),\qquad
   \bm{y}_{n+1}=\bm{y}_n+h\sum_{i=1}^{12}b_i\bm{k}_i

係数 :math:`c_i,\,a_{ij},\,b_i` は ``dop853.f`` の値をそのまま写したものである（:math:`c_{12}=1`）。:math:`b_i` は :math:`i=1,6,\dots,12` だけが 0 でない。
係数が正しいことは、SciPy の係数表の 76 個の定数と :math:`10^{-15}` で一致すること、
:math:`y'=y\cos t` の固定刻みの積分で観測される次数が 8 であることで確かめている（:doc:`verification`）。
密な出力（dense output）の係数は使わない。ランチャ離脱・頂点・着地の時刻は ``rk45`` と同じく、ステップ内の補間で求める。

誤差の推定
^^^^^^^^^^

誤差の推定には、5 次と 3 次の 2 つの埋め込み解を使う。``dop853.f`` の係数 :math:`\mathrm{er}_i` と、
8 次の重み :math:`b_i` から 3 次の解の重み :math:`\widehat b_i`\ （``bhh``。:math:`i=1,9,12` だけが 0 でない）を引いた :math:`b_i-\widehat b_i` を用いて、

.. math::
   :label: eq-dop853-e

   \bm{e}_5=h\sum_{i=1}^{12}\mathrm{er}_i\,\bm{k}_i,\qquad
   \bm{e}_3=h\sum_{i=1}^{12}\left(b_i-\widehat b_i\right)\bm{k}_i

とする。:math:`\bm{e}_5,\bm{e}_3` の各成分 :math:`c` を :math:`\mathrm{sc}_c=\varepsilon_a+\varepsilon_r\max(|y_{n,c}|,\,|y_{n+1,c}|)` で割り、
:math:`n=13` 個の成分の二乗平均平方根を :math:`\mathrm{err}_5,\ \mathrm{err}_3` とする。
実装では、:math:`h` を除いた二乗和 :math:`S_5=\sum_c\bigl(\sum_i\mathrm{er}_ik_{i,c}/\mathrm{sc}_c\bigr)^2`、
:math:`S_3=\sum_c\bigl(\sum_i(b_i-\widehat b_i)k_{i,c}/\mathrm{sc}_c\bigr)^2` から

.. math::
   :label: eq-dop853-err

   \mathrm{err}=\frac{|h|\,S_5}{\sqrt{n\,\bigl(S_5+0.01\,S_3\bigr)}}
   =\frac{\mathrm{err}_5^{\,2}}{\sqrt{\mathrm{err}_5^{\,2}+0.01\,\mathrm{err}_3^{\,2}}}

を求める。5 次の推定を 3 次の推定で補正した形である。
刻み幅が小さいとき :math:`\mathrm{err}_5\propto h^6`、:math:`\mathrm{err}_3\propto h^4` なので、:math:`\mathrm{err}\propto h^8` となり、
刻み幅の制御に指数 :math:`1/8` を使う根拠になる。

刻み幅の制御
^^^^^^^^^^^^

:math:`\mathrm{err}\le1` ならステップを受理し、次の刻み幅（やり直す場合はそのときの刻み幅）に

.. math::
   :label: eq-dop853-factor

   h\leftarrow h\cdot f,\qquad f=\min\!\left(6,\ \max\!\left(0.333,\ 0.9\,\mathrm{err}^{-1/8}\right)\right)

を掛ける。安全率は 0.9、倍率の範囲は 0.333〜6 で、:math:`\mathrm{err}=0` のときは 6 である。
``dop853.f`` にある、直前のステップの誤差を使う安定化（:math:`\beta`）は使わない。刻み幅の上限と下限の扱いは ``rk45`` と同じである。
12 段の計算の後に、受理された点での右辺 :math:`\bm{k}_{13}=\bm{f}(t+h,\bm{y}_{n+1})` を 1 回評価し、次のステップの :math:`\bm{k}_1` として再利用する
（FSAL）。:math:`\bm{k}_1` は前のステップの値を使うので、1 ステップで新たに評価する右辺は :math:`\bm{k}_2,\dots,\bm{k}_{12}` の 11 回と :math:`\bm{k}_{13}` の 1 回の計 12 回で、``rk45`` の 6 回（:math:`\bm{k}_2,\dots,\bm{k}_7`）の 2 倍である。
段階が切り替わったステップの後は ``rk45`` と同じく :math:`\bm{k}_1` を評価し直す。
``attitude = "normalize"`` では、:math:`\bm{k}_{13}` はクォータニオンを正規化する前の :math:`\bm{y}_{n+1}` で評価する（正規化による差は許容誤差の程度である）。

``dop853`` が有利になる場合
^^^^^^^^^^^^^^^^^^^^^^^^^^^

高次の方法は、解が滑らかで許容誤差が厳しいときに、少ない評価回数で同じ精度に達する。
空力係数が Mach 数に依らず迎角に線形で、風がなく、推力が 0 から始まって 0 で終わる滑らかな弾道飛翔（出力間隔 0.5 s）では、
``rk45`` と ``dop853`` の右辺の評価回数は、``rtol = atol = 1e-9`` で 7535 回と 4205 回、``1e-11`` で 17981 回と 6353 回であった（実測）。
許容誤差が厳しいほど差は開く。

一方、サンプルの設定ではこの利点は出ない（:numref:`tbl-integrator-attitude`）。
空力係数表は区分的に線形なので、Mach 数と迎角が格子点をまたぐところで右辺の微分が不連続になり、高次の方法でも刻み幅は大きくできない。
さらに、刻み幅は出力間隔（0.05 s）で頭打ちになる。降下中は解が滑らかなので ``rk45`` でも誤差の条件より先にこの上限に達し、
ステップ数は ``rk45`` が約 1.39 万、``dop853`` が約 1.38 万とほとんど変わらない。1 ステップの評価回数が 12 回と 6 回なので、``dop853`` の評価回数は約 2 倍になる。
このサンプルでは、``rk45`` のほうが評価回数も時間も少ない。

Lie 群による姿勢の積分（``attitude = "lie_group"``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

既定の ``attitude = "normalize"`` は、クォータニオンの 4 成分を通常の状態量として積分し、1 ステップごとに正規化する。
``"lie_group"`` では、姿勢を単位クォータニオンのなす群 :math:`S^3`\ （回転群 SO(3) の 2 重被覆）の上で直接進める。
Munthe-Kaas の Runge–Kutta–Munthe-Kaas（RKMK）法 :cite:`munthekaas98,munthekaas99` で、Lie 群上の積分の総説は :cite:`iserles,hairergni` にある。
更新は常に単位クォータニオンどうしの積なので、正規化をしなくても :math:`|\bm{q}|` は丸め誤差の範囲で 1 に保たれる。

指数写像と対数写像
^^^^^^^^^^^^^^^^^^

Lie 環 so(3) を :math:`\mathbb{R}^3` と同一視し、括弧積を外積とする。回転ベクトル :math:`\bm\theta`\ （rad）に対して

.. math::
   :label: eq-lie-exp

   \exp(\bm\theta)=\left(\cos\frac{|\bm\theta|}2,\ \sin\frac{|\bm\theta|}2\,\frac{\bm\theta}{|\bm\theta|}\right)

は、:math:`\bm\theta` まわりに角度 :math:`|\bm\theta|` だけ回す単位クォータニオンである。逆写像は、:math:`\bm{q}=(q_w,\bm{q}_v)` を :math:`q_w\ge0` となる符号にとって

.. math::
   :label: eq-lie-log

   \log(\bm{q})=\frac{2\,\mathrm{atan2}(|\bm{q}_v|,\,q_w)}{|\bm{q}_v|}\,\bm{q}_v

で、角度が :math:`[0,\pi]` の最短の回転を与える（:math:`\bm{q}` と :math:`-\bm{q}` は同じ回転）。
:math:`|\bm\theta|` が小さいときは、:math:`\cos(|\bm\theta|/2)` と :math:`\sin(|\bm\theta|/2)/|\bm\theta|` を Taylor 展開で評価する。

指数写像の微分
^^^^^^^^^^^^^^

運動方程式 :math:`\dot{\bm{q}}=\frac12\bm{q}\otimes(0,\bm{\omega})` を、:math:`\bm{q}=\bm{q}_n\otimes\exp(\bm\Theta(t))` と置いて :math:`\bm\Theta` の方程式に直す。
:math:`\exp(\bm{u})^{-1}\,\mathrm{d}\exp(\bm{u}+\epsilon\bm{v})/\mathrm{d}\epsilon=\mathrm{dexp}_{-\bm{u}}(\bm{v})` なので、

.. math::
   :label: eq-lie-theta

   \dot{\bm\Theta}=\mathrm{dexp}^{-1}_{-\bm\Theta}(\bm{\omega})

となる。so(3) では :math:`\mathrm{dexp}` とその逆が閉じた形で書ける（:math:`s=|\bm{u}|`）。

.. math::
   :label: eq-lie-dexp

   \mathrm{dexp}_{\bm{u}}(\bm{v})=\bm{v}+\frac{1-\cos s}{s^2}\,\bm{u}\times\bm{v}+\frac{s-\sin s}{s^3}\,\bm{u}\times(\bm{u}\times\bm{v})

.. math::
   :label: eq-lie-dexpinv

   \mathrm{dexp}^{-1}_{\bm{u}}(\bm{v})=\bm{v}-\frac12\,\bm{u}\times\bm{v}+\frac1{s^2}\left(1-\frac s2\cot\frac s2\right)\bm{u}\times(\bm{u}\times\bm{v})

これらの係数は :math:`s\to0` で 0 を 0 で割る形になるので、:math:`s<0.1` のときは級数

.. math::
   :label: eq-lie-series

   \frac{1-\cos s}{s^2}&=\frac12-\frac{s^2}{24}+\frac{s^4}{720}-\cdots,\\
   \frac{s-\sin s}{s^3}&=\frac16-\frac{s^2}{120}+\frac{s^4}{5040}-\cdots,\\
   \frac1{s^2}\left(1-\frac s2\cot\frac s2\right)&=\frac1{12}+\frac{s^2}{720}+\frac{s^4}{30240}+\cdots

で評価する。:math:`\mathrm{dexp}^{-1}` は :math:`s=2\pi` で特異になるが、1 ステップの回転角はそれよりずっと小さい。

RKMK 法の 1 ステップ
^^^^^^^^^^^^^^^^^^^^

状態を、ユークリッド空間の部分 :math:`\bm{x}=(\bm{x}_{\mathrm{pos}},\bm{v},\bm{\omega})`\ （9 成分）と姿勢 :math:`\bm{q}` に分ける。
右辺を :math:`(\dot{\bm{x}},\bm{\omega})=\bm{f}(t,\bm{x},\bm{q})` として、Butcher 表 :math:`(c_i,a_{ij},b_i)` の方法の 1 ステップは次のとおりである。

.. math::
   :label: eq-rkmk-stage

   \begin{aligned}
   \bm\Theta_i&=h\sum_{j<i}a_{ij}\widetilde{\bm{K}}_j, &
   \bm{X}_i&=\bm{x}_n+h\sum_{j<i}a_{ij}\bm{k}_j, &
   \bm{Q}_i&=\bm{q}_n\otimes\exp(\bm\Theta_i),\\
   (\bm{k}_i,\bm{\omega}_i)&=\bm{f}(t+c_ih,\ \bm{X}_i,\ \bm{Q}_i), &
   \widetilde{\bm{K}}_i&=\mathrm{dexp}^{-1}_{-\bm\Theta_i}(\bm{\omega}_i)
   \end{aligned}

.. math::
   :label: eq-rkmk-update

   \bm{x}_{n+1}=\bm{x}_n+h\sum_ib_i\bm{k}_i,\qquad
   \bm\Theta=h\sum_ib_i\widetilde{\bm{K}}_i,\qquad
   \bm{q}_{n+1}=\bm{q}_n\otimes\exp(\bm\Theta)

角速度 :math:`\bm\omega` はユークリッド部分 :math:`\bm{x}` の成分でもあるので、Euler の運動方程式（:eq:`eq-euler`）の積分は通常の Runge–Kutta 法と同じで、
各段の姿勢 :math:`\bm{Q}_i` を右辺の空力に渡す点だけが違う。:eq:`eq-lie-dexpinv` の閉じた形をそのまま使うので、表の方法の次数は保たれる。
自由な剛体（慣性主軸のモーメント :math:`I=\mathrm{diag}(1,2,3)`、:math:`\bm\omega_0=(0.3,\,1.0,\,0.4)` rad/s、トルクなし、:math:`t\in[0,20]` s）で
観測した次数は、RKMK 法の ``rk4`` の表で 4.10、Dormand–Prince 5 次の表で 4.92 である。

.. _fig-rigid-body-attitude:

.. figure:: _generated/plots/rigid_body_attitude.*
   :width: 100%

   トルクのない剛体（:math:`I=\mathrm{diag}(1,2,3)`、刻み :math:`h=0.05` s）の姿勢の誤差（左）とクォータニオンのノルムの誤差（右）。
   参照解は RKMK 法（Dormand–Prince 5 次、:math:`h=1` ms）である。RK4 の「正規化前」は、毎ステップの正規化で取り除かれる偏差を示す。値がちょうど 0 の点は対数軸に描けないので省いた。

同じ問題の結果を :numref:`fig-rigid-body-attitude` に示す。刻み :math:`h=0.01` s の RKMK 法（``rk4`` の表）では、:math:`\bigl||\bm{q}|-1\bigr|` の最大値は :math:`5.8\times10^{-15}`、
エネルギーの相対誤差は :math:`7.1\times10^{-12}`、角運動量の大きさの相対誤差は :math:`4.6\times10^{-12}` である（実測）。
:math:`h=0.05` s では、姿勢の誤差は RKMK 法が :math:`1.57\times10^{-7}` rad、RK4 と正規化が :math:`2.59\times10^{-7}` rad で、同程度である。
この問題では、通常の状態量として積分した RK4 は 1 ステップごとに最大 :math:`1.9\times10^{-10}` のノルムの偏差を生じ、正規化がそれを取り除いている。

誤差の推定と刻み幅の制御
^^^^^^^^^^^^^^^^^^^^^^^^

適応的な方法（``rk45``、``dop853``）では、埋め込み解の重み :math:`e_i` を、ユークリッド部分の :math:`\bm{k}_i` と、姿勢の Lie 環の増分 :math:`\widetilde{\bm{K}}_i` の両方に掛ける。

.. math::
   :label: eq-lie-err

   \bm{e}_x=h\sum_ie_i\,\bm{k}_i,\qquad
   \bm\Theta_e=h\sum_ie_i\,\widetilde{\bm{K}}_i

``rk45`` では :math:`e_i=E_i`\ （:eq:`eq-butcher-e`）、``dop853`` では :eq:`eq-dop853-e` の 2 組の重みを使う。
:math:`\bm\Theta_e` は回転ベクトル（rad）で、2 つの解の姿勢の間の角度に（1 次の精度で）等しい。
ユークリッド部分の 9 成分は :math:`\varepsilon_a+\varepsilon_r\max(|x_{n,c}|,|x_{n+1,c}|)` で、姿勢の 3 成分は :math:`\varepsilon_a+\varepsilon_r\pi` で割る。
許容誤差が小さいときは、姿勢の誤差が :math:`\varepsilon_a` rad 程度まで許される。
計算した 12 成分から ``rk45`` は :eq:`eq-dopri-err` と同じ二乗平均平方根を、``dop853`` は :eq:`eq-dop853-err` を（:math:`n=12`）求める。
刻み幅の制御は、それぞれの方法のとおりである。

座標系と段階
^^^^^^^^^^^^

運動方程式 :math:`\dot{\bm{q}}=\frac12\bm{q}\otimes(0,\bm\omega)` の :math:`\bm\omega` は、積分する座標系に対する機体座標での角速度で、状態量の :math:`\bm\omega` そのものである。
平面地球では座標系は回転しない。ECEF では :math:`\bm{q}` は機体から ECEF への姿勢で、:math:`\bm\omega` は ECEF に対する角速度である。
地球の自転 :math:`\bm\Omega_\oplus` の項は :math:`\dot{\bm\omega}` の方程式にだけ現れ（:eq:`eq-ecef-euler`）、:math:`\dot{\bm{q}}` には現れない。
したがって、どちらの座標系でも Lie 環の元は各段の :math:`\bm\omega_i` である。

ランチャ上とパラシュート降下では姿勢の運動を解かず、:math:`\bm{q}` は一定である。このとき Lie 環の元を :math:`\bm\omega_i=\bm{0}` とすると、
:math:`\bm\Theta_i=\bm\Theta=\bm{0}` となって :math:`\bm{q}` はまったく変わらず、ユークリッド部分は同じ Butcher 表で進む。

積分法と姿勢の更新法の組合せ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

積分法は ``sim.integrator``\ （``"rk4"``、``"rk45"``、``"dop853"``）、姿勢の更新法は ``sim.attitude``\ （``"normalize"``、``"lie_group"``）で選び、
6 通りの組合せがすべて使える。:numref:`tbl-integrator-attitude-keys` に仕組みを、:numref:`tbl-integrator-attitude` にサンプルの結果をまとめる。

.. _tbl-integrator-attitude-keys:

.. list-table:: 積分法と姿勢の更新法
   :header-rows: 1
   :widths: 20 20 20 20 20

   * - ``sim.integrator``
     - 刻み幅
     - 1 ステップの評価回数
     - 誤差の成分数（``normalize`` / ``lie_group``）
     - 姿勢の扱い
   * - ``rk4``
     - 固定（``dt``）
     - 4
     - なし
     - ``normalize``: 毎ステップ正規化、``lie_group``: :eq:`eq-rkmk-update`
   * - ``rk45``
     - 制御（``rtol``、``atol``）
     - 6（FSAL）
     - 13 / 12
     - 同上
   * - ``dop853``
     - 制御（``rtol``、``atol``）
     - 12（FSAL）
     - 13 / 12
     - 同上

.. _tbl-integrator-attitude:

.. list-table:: 積分法と姿勢の更新法の比較（サンプル、パラシュート降下、既定の許容誤差、単一スレッド）
   :header-rows: 1
   :widths: 30 20 25 25

   * - 積分法 / 姿勢
     - 頂点 [m]
     - 評価回数
     - 時間 [ms]
   * - ``rk4`` / ``normalize``
     - 4439.946
     - 1 369 780
     - 175
   * - ``rk45`` / ``normalize``
     - 4439.946
     - 83 116
     - 18
   * - ``rk45`` / ``lie_group``
     - 4439.946
     - 83 092
     - 23
   * - ``dop853`` / ``normalize``
     - 4439.943
     - 165 022
     - 44
   * - ``dop853`` / ``lie_group``
     - 4439.943
     - 164 950
     - 42
   * - ``rk4`` :math:`\Delta t=0.5` ms（参照）
     - 4439.945
     - ―
     - ―

どの組合せも、頂点高度は参照解と 0.002 m 以内で一致する。``lie_group`` は、``normalize`` に比べて ``rk45`` で時間が 5 ms（約 30 %）増える。
サンプルでは、姿勢の更新法による頂点高度の差は 0.001 m 以下で、計算時間は、``rk45`` では ``lie_group`` のほうが長く、``dop853`` ではほぼ同じである。
``lie_group`` は、正規化に頼らず四元数のノルムを 1 に保ちたいときに選ぶ。

計算例
------

付属のサンプル（射角 85°、方位 270°、北風 4 m/s、パラシュートあり）の結果を示す。

.. _fig-trajectory:

.. figure:: _generated/plots/trajectory.*
   :width: 100%

   高度の時刻歴（左）と地表への投影（右）。

.. _fig-flight-states:

.. figure:: _generated/plots/flight_states.*
   :width: 100%

   上昇中の Mach 数、迎角、静安定余裕（その時刻の迎角での表示 ``stability_cal``）、動圧。

.. list-table:: サンプルの主な結果
   :header-rows: 1
   :widths: 50 50

   * - 量
     - 値
   * - ランチャ離脱速度
     - 45.7 m/s（0.25 s）
   * - 最大速度 / 最大 Mach 数
     - 518.7 m/s / 1.54
   * - 最大動圧
     - 152 kPa
   * - 最小静安定余裕（小迎角の余裕）
     - 1.99 cal（ランチャ離脱時、0.25 s、Mach 0.13）
   * - 頂点
     - 4440 m（26.1 s）
   * - 着地
     - 685 s、射点から 7.7 km

判定に使う小迎角の余裕は、ランチャ離脱の時刻に最小になる。燃焼中は推進剤の消費で重心が前に移るので、余裕はそこから増え続け、
超音速ではフィンの圧力中心も後退するので、燃焼の終わりごろ（3.45 s、Mach 1.42）に 3.6 cal に達する。
ランチャ離脱直後は速度が小さく、横風による迎角が大きい（約 6°）。迎角が大きいと胴体揚力（:eq:`eq-body-lift`）が圧力中心を前に移すので、
:numref:`fig-flight-states` のその時刻の迎角での表示 ``stability_cal`` は、このとき 1.58 cal まで下がる。この表示は判定には使わない（:ref:`sec-static-margin-general`）。
