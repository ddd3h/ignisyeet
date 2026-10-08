.. _page-theory-dynamics:

6 自由度の運動方程式
======================

飛翔中の機体の運動を、並進（重心の運動）と回転（姿勢の運動）の連立方程式で表す。
この章は、その式と、設定で選べる運動モデルの違い（準定常と非定常の項）を説明する。
降下の方式は :ref:`page-theory-descent`、数値積分の方法は :ref:`page-theory-integrators`、
風や大気の入力は :ref:`page-theory-wind` と :ref:`page-theory-atmosphere-gravity` に述べる。

この章で決めること
------------------

運動方程式そのものに選択肢はなく、飛翔は常に 6 自由度で計算する。設定で決めるのは、次の三つである。

* 角速度・角加速度に比例する非定常な空気力の項を飛翔に入れるか：:ref:`sim.quasi_steady <cfg-sim-quasi_steady>`
* 落下分散で、非定常な項と減衰の不確かさをどれだけ見込むか：
  :ref:`lag_scale <cfg-dispersion-monte_carlo-lag_scale>`、:ref:`magnus_scale <cfg-dispersion-monte_carlo-magnus_scale>`、:ref:`cmq_scale <cfg-dispersion-monte_carlo-cmq_scale>`
* 係数表の種類（軸対称か一般か）：:ref:`aero.table_kind <cfg-aero-table_kind>`。判断は :ref:`page-theory-aero-tables` にある。

機体の慣性（:ref:`rocket.iyy_dry <cfg-rocket-iyy_dry>` など）と推力線（:ref:`motor.misalignment_deg <cfg-motor-misalignment_deg>` など）の入力は、
この章の式（:ref:`sec-inertia-tensor`、:ref:`sec-thrust-line`）で運動に入る。
結果が使えるかどうかの目安になる静安定余裕の読み方も、この章に置く（:ref:`sec-static-margin-general`）。

選び方
------

非定常な項（見かけの質量の遅れ、回転による法線力、Magnus 力）を切るかどうかは、``sim.quasi_steady`` で決める。
既定は ``false``\ （すべて含む）である。:numref:`fig-quasi-steady-schematic` は、この設定で切れる項と、常に残る項を示す。

.. _fig-quasi-steady-schematic:

.. figure:: /_generated/tikz/quasi_steady_schematic.*
   :width: 100%

   ``sim.quasi_steady`` で切れる項。左の列が既定（すべて含む）、右の列が準定常である。橙色の 3 行が切れる項で、上の 4 行は準定常でも残る。
   見る点は、切れるのが角速度・角加速度に比例する項だけで、静的な係数、減衰、ロールの項は常に残ることである。

:numref:`tbl-choose-quasi-steady` に、切れる項と、不確かさの与え方をまとめる。
式と記号は :ref:`sec-aero-unsteady` にある。

.. _tbl-choose-quasi-steady:

.. list-table:: 非定常な項の扱い
   :header-rows: 1
   :widths: 16 20 20 24 20

   * - 項
     - 表すもの
     - 値の出どころ
     - 影響が大きい機体
     - 不確かさの与え方
   * - 見かけの質量の遅れと対流の回転
     - 横向きの加速度や角速度に遅れて付いてくる空気の反作用
     - 表の ``lag_l0``、``lag_l1``。部品積み上げ法、パネル法、CFD の表は形状から作る
     - 胴体とフィンが大きく、見かけの質量が大きい機体。サンプルではピッチ減衰の 16〜29 % を占める
     - ``lag_scale``。危険区域の解析では標準偏差 0.25
   * - 回転による法線力
     - 回転で機尾が横へ動くことで生じる力（減衰のモーメントと対になる）
     - 減衰の和 :math:`S_0,S_1`
     - 影響は小さい。サンプルの地上付近でピッチの固有振動数の 2 乗を 0.3〜0.5 % 変える程度
     - ``cmq_scale`` に重ねて掛かる
   * - Magnus 力とモーメント
     - ロールしている機体が迎角を持つときの横力
     - 表の ``cnpa``、``cmpa``。組み込みの手法は推算せず 0 にする（外部の表で与える）
     - 速くロールして迎角を持つ機体で、外部の表に値がある場合
     - ``magnus_scale``。表に値がなければ効かず、警告を出す
   * - ピッチ・ヨーの減衰（準定常でも残る）
     - 局所的な迎角の和による減衰のモーメント
     - 減衰の和 :math:`S_0,S_1,S_2`
     - すべての機体
     - ``cmq_scale``。危険区域の解析では標準偏差 0.1

判断の目安は次のとおりである。

* **既定の** ``false`` **で飛ばす。** 非定常な項は、角速度の小さい通常の飛翔では着地点にほとんど効かない（:numref:`tbl-quasi-steady-effect`）。
  効くのは迎角の大きさで、サンプルでは上昇中の最大迎角が約 8 % 変わる。
* ``true`` は感度の確認用である。``false`` との差が、保安距離に見込む余裕より十分に小さいことを確かめるために使う。
* 外部の表に Magnus の値を入れた機体では ``false`` のままにし、値の不確かさを ``magnus_scale`` で見込む。
* 落下分散では、``cmq_scale`` と ``lag_scale`` を別々に与える。``cmq_scale`` は局所的な迎角の和だけに掛かり、
  ピッチ減衰の 16〜29 %（サンプル）を占める見かけの質量の項には掛からないからである（:ref:`sec-quasi-steady`）。
  ``magnus_scale`` は、表に ``cnpa``、``cmpa`` の値があるときだけ与える。

.. _tbl-quasi-steady-effect:

.. list-table:: ``sim.quasi_steady`` の影響（サンプル）
   :header-rows: 1
   :widths: 34 12 12 14 14 14

   * - 条件
     - 設定
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
     - 上昇中の最大迎角 [deg]
   * - パラシュート降下
     - ``false``
     - 4440.48
     - −655.57
     - −7707.70
     - 1.67
   * -
     - ``true``
     - 4440.71
     - −654.73
     - −7708.12
     - 1.54
   * - 弾道落下
     - ``false``
     - 4440.48
     - −1072.87
     - 372.30
     - ―
   * -
     - ``true``
     - 4440.71
     - −1071.82
     - 372.72
     - ―
   * - カント 1°、乾燥重心の横ずれ 1 mm（パラシュート降下）
     - ``false``
     - 4425.45
     - −686.66
     - −7589.36
     - 1.29
   * -
     - ``true``
     - 4425.59
     - −685.94
     - −7589.17
     - 1.18

条件は ``examples/sample.toml``\ （射角 85°、射方位 270°、北風 4 m/s のべき法則の風）に ``sim.integrator = "rk45"`` を加えたもので、
最大迎角は時刻 1〜30 s の最大値である。最後の 2 行は ``rocket.fin_cant_deg = 1.0`` と ``rocket.cg_lateral_dry = [0.001, 0.0]`` を加え、
ロール共振の警告（IY-O510）が出る飛翔である。
組み込みの表では ``cnpa``、``cmpa`` が 0 なので、この比較で変わるのは見かけの質量の項と回転による法線力だけである。
どの条件でも、着地点の東西と南北の差は 1.1 m 以下、頂点の差は 0.25 m 以下で、最大迎角の差は約 8 % である。

静安定余裕の見方
~~~~~~~~~~~~~~~~

飛翔の結果が使えるかどうかは、小迎角の静安定余裕で判定する（定義は :ref:`sec-static-margin-general`）。
:numref:`tbl-choose-margin` の区分で警告と終了ステータスが決まる。

.. _tbl-choose-margin:

.. list-table:: 静安定余裕の区分
   :header-rows: 1
   :widths: 24 28 48

   * - 静安定余裕
     - 結果の表示
     - 意味と対応
   * - 1.5 cal 以上
     - 警告なし
     - 十分な余裕がある。
   * - 1.0 cal 以上 1.5 cal 未満
     - ``marginal (< 1.5 cal)``
     - 余裕が小さい。突風や重心のずれで不安定に近づく。
   * - 0 cal 以上 1.0 cal 未満
     - ``low static margin``
     - 余裕が不足している。フィンを大きくするか、重心を前へ寄せる。
   * - 0 cal 未満
     - ``UNSTABLE``
     - 圧力中心が重心より前にある。結果は使えず、終了ステータスは 3 になる（:ref:`sec-exit-status`）。

サンプルの最小の余裕は、ランチャ離脱の時刻（0.25 s、Mach 0.13）の 1.99 cal である。
燃焼中は推進剤の消費で重心が前へ移るので、余裕はそこから増える（:ref:`fig-flight-states`）。
離脱速度が小さい機体では、風による迎角が大きく、その迎角での圧力中心の表示 ``stability_cal`` は判定値より小さくなる。
判定には入らないので、風による迎角の警告（:ref:`sec-config-checks`）と合わせて見る。

モデル
------

状態量
~~~~~~

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
~~~~~~~~

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

係数表には **軸対称表** と **一般表** があり、設定 ``aero.table_kind`` で選ぶ（概念は :ref:`sec-aero-table`、係数の定義は :ref:`page-theory-aero-coefficients`）。
飛翔計算は、表の種類に応じて次の二つの式で空気力と空力モーメントを求める。

対気速度を機体座標で表したものを :math:`\bm{u}=R^\top(\bm{v}-\bm{w})=(u_x,u_y,u_z)` とし、

.. math::
   :label: eq-alpha

   V=|\bm{u}|,\qquad u_\perp=\sqrt{u_y^2+u_z^2},\qquad
   \alpha_t=\operatorname{atan2}(u_\perp,\,u_x),\qquad M=\frac{V}{a(h_0+z_U)}

とする。:math:`\alpha_t` はピッチとヨーを合わせた **全迎角**\ （:math:`0\le\alpha_t\le\pi`）である。
機体座標での迎角 :math:`\alpha` と横滑り角 :math:`\beta` は :ref:`sec-aero-angles` で定義する。

軸対称表
~~~~~~~~

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
~~~~~~

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

である。静的な安定は、角度を戻す向きのモーメントの傾き（復元の微係数）\ :math:`R` の符号で決まる。
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

余裕は、すべての飛翔（``sim``、``dispersion`` の公称の飛翔、風の格子のケース、モンテカルロのサンプル）について求める。
自由飛行のうち頂点より前で対気速度が 10 m/s を超える間の各時刻と、ランチャ離脱の時刻に、その時刻の Mach 数と重心で 2 つの余裕を求める。
ランチャ離脱の時刻の余裕は、対気速度によらず求め、離脱時の対気速度が 10 m/s を超えるときは自由飛行の最小値にも入れる。
軌道の出力間隔（``sim.output_interval``）には依らない。

* ``min_stability_cal`` と ``stability_at_rail_exit`` は、その時刻のピッチとヨーの余裕の小さいほう（求まったもの）の、それぞれ最小値とランチャ離脱時の値である。頂点までに対気速度が 10 m/s を超えない飛翔では ``min_stability_cal`` は ``null`` で、判定（``static_margin_min_cal``）にはランチャ離脱時の値を使う。
* ``static_margin_pitch_min_cal`` と ``static_margin_yaw_min_cal`` は、面ごとの最小値（自由飛行とランチャ離脱を合わせたもの）である。どこかで余裕が求まらなかった面は ``null`` になる。
* 余裕が求まらない時刻があれば、最初のものを ``at t = 0.50 s: …`` の形で ``stability_unknown`` に残し、静的な安定は「判定できない」とする（:ref:`sec-exit-status`）。求まった余裕に負のものがあれば、判定できない余裕があっても不安定とする。
* 最小の余裕をとった状態の時刻と Mach 数を ``static_margin_min_t`` と ``static_margin_min_mach`` に残す（余裕を一度も求めなかった飛翔では ``null``）。
  結果パネルの ``min margin`` の行、プレーンモードの ``min static margin`` の行、静安定余裕の警告と ``RESULTS NOT USABLE`` の文には、これを ``at t = 0.25 s, Mach 0.13`` の形で添える（数はサンプルの値）。

回転運動
~~~~~~~~

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
点質量 :math:`m` を :math:`(x,y,0)` に置くと :math:`I_{xy}=-mxy` になる。

乾燥時の慣性テンソルは、乾燥重心まわりの値を設定で与える。
推進剤を足した現在の慣性テンソルは、まず質量の中心を求め、各部分の慣性テンソルを平行軸の定理で現在の重心まわりに移して足す。

.. math::
   :label: eq-inertia-parallel

   I = I_d + m_d\left(|\bm{r}_d|^2E-\bm{r}_d\bm{r}_d^\top\right)
     + I_p + m_p\left(|\bm{r}_p|^2E-\bm{r}_p\bm{r}_p^\top\right)

:math:`E` は単位行列、:math:`\bm{r}_d,\bm{r}_p` は現在の重心から乾燥部分の重心と推進剤の重心までの機体座標のベクトルである。
推進剤は軸上にあるので、乾燥重心の横ずれを :math:`(y_d,z_d)` とすると、重心は :math:`y_{cg}=m_dy_d/m,\ z_{cg}=m_dz_d/m` にずれる。
推進剤の円柱の慣性テンソルは、:math:`I_p=\operatorname{diag}\!\left(\tfrac12m_pr_m^2,\ m_p(3r_m^2+L_m^2)/12,\ m_p(3r_m^2+L_m^2)/12\right)` である（:ref:`page-theory-propulsion`）。

慣性乗積も重心の横ずれもなく :math:`I_{zz}=I_{yy}` のとき、慣性テンソルは :math:`\operatorname{diag}(I_{xx},I_{yy},I_{yy})` になり、
:eq:`eq-euler` を成分ごとの式として解く。それ以外では、逆行列を使って

.. math::
   :label: eq-euler-tensor

   \dot{\bm{\omega}} = I^{-1}\left(\bm{M}_b-\bm{\omega}\times(I\bm{\omega})\right)

とする。飛翔の前に、乾燥時と燃焼中の各時刻の重心まわりの :math:`I` の固有値（主慣性モーメント）を求め、
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

* 見かけの質量の遅れ（:math:`\dot\alpha`）と対流の回転によるモーメントと、対流の回転の力（表の ``lag_l0``、``lag_l1``）
* 回転による法線力（:math:`C_{Nq}`。減衰の和 :math:`S_0,S_1` から）
* Magnus 力とモーメント（表の ``cnpa``、``cmpa``）
* 誘導ロール（表の ``cl4``）

``sim.quasi_steady = true``\ （:ref:`[sim] の設定項目 <sec-config-sim>`）にすると、誘導ロールを除くこれらの項を切り、静的な係数と減衰のモーメントとロールの項だけで飛ばす（感度の確認用）。

どれも状態量を増やさず、そのときの状態とその微分だけで決まる（準定常。:ref:`sec-quasi-steady`）。
表の列は :ref:`sec-file-axisym` に、各手法での値の出どころは :ref:`sec-aero-damping` にある。

見かけの質量の遅れと対流の回転
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

細長い物体の理論（slender-body theory）\ :cite:`munk,nielsenbook` では、機体の断面は、断面積 :math:`A(x)` に比例する空気の質量 :math:`\rho A(x)`\ （単位長さあたり）を横向きの運動に連れて動かす（見かけの質量）。
胴体の断面は :math:`A=\pi r^2`、フィンのある断面は胴体を含めて :math:`A=\pi(s^2-r^2+r^4/s^2)`\ （:math:`s` は軸からのフィンの半幅）である。
ノーズ先端から後方へ :math:`x`、重心からの距離を :math:`l=x-x_{cg}` とし、ピッチ面で書くと、断面の横向きの対気速度は :math:`w_c=u_z+\omega_y\,l` で（:math:`\omega_y>0` で機尾は :math:`+z_b` へ動く）、断面に働く力は

.. math::
   :label: eq-lag-dyn-section

   f(x)=-\rho\,\frac{D}{Dt}\bigl[A\,w_c\bigr],\qquad \frac{D}{Dt}=\frac{\partial}{\partial t}+V\frac{\partial}{\partial x},\qquad
   \frac{D}{Dt}\bigl[A\,w_c\bigr]=A\,(\dot u_z+l\,\dot\omega_y)+V A'\,(u_z+\omega_y\,l)+V A\,\omega_y

である（:math:`A'=\mathrm{d}A/\mathrm{d}x`）。右辺の各項の扱いは次のとおりである。

* :math:`V A' u_z`：静的な法線力。係数表の :math:`C_N` に入っている（胴体は Munk の法線力、フィンは Barrowman の法線力）。
* :math:`V A' \omega_y\,l`：回転による局所的な迎角の項。減衰の和 :math:`S_0,S_1,S_2`\ （:eq:`eq-damping`）に入っている（フィンは Barrowman の圧力中心で）。
* :math:`A\,\dot u_z`：横向きの対気速度の変化に遅れて付いてくる空気の反作用（:math:`\dot\alpha` の項、遅れの項）。この節で加える。
* :math:`V A\,\omega_y`：対流による回転の項。局所的な迎角の和は :math:`A'` の項だけを数えるので、この項は減衰の和に入っていない。この節で加える。
* :math:`A\,l\,\dot\omega_y`：角加速度による付加慣性。省く。

:math:`A\dot u_z` と :math:`VA\omega_y` の項は、どちらも :math:`A` に比例する。係数表には、これを基準面積で無次元化した遅れの和

.. math::
   :label: eq-lag-dyn-sums

   L_0=\frac{2}{S}\int A\,\mathrm{d}x\ [\mathrm{m}],\qquad L_1=\frac{2}{S}\int A\,x\,\mathrm{d}x\ [\mathrm{m^2}]

を Mach 数ごとに持つ（``lag_l0``、``lag_l1``）。胴体の分は容積 :math:`\mathrm{Vol}` とその中心 :math:`x_{\mathrm{vol}}` から :math:`2\mathrm{Vol}/S` と :math:`2\mathrm{Vol}\,x_{\mathrm{vol}}/S`、
フィンの分はフィンの断面の増分をフィンの法線力の傾きにそろえたものである（部品積み上げ法での求め方は :ref:`sec-damping`）。

部分積分すると、ピッチ振動（:math:`\dot u_z=V\omega_y`）でのこれらの項の和は機体の後端の断面だけで決まり、

.. math::
   :label: eq-sbt-damping

   M=-\rho V\omega_y\,A_e\,l_e^2,\qquad C_{mq}+C_{m\dot\alpha}=-\frac{4A_el_e^2}{Sd^2}=-2\,C_{N\alpha}\left(\frac{l_e}{d}\right)^2

になる（:math:`A_e` と :math:`l_e` は後端の断面と重心からの距離、:math:`C_{N\alpha}=2A_e/S`）。
飛翔計算は Barrowman の局所的な迎角の和と遅れの和からこの減衰を組み立て、サンプルでは閉じた式と 6〜7 % 以内で一致する（:ref:`sec-verify-unsteady`）。
閉じた式の :math:`C_{N\alpha}` もフィンの見かけの質量も Barrowman の傾きによるので、この一致は項の組み立ての整合性を示し、減衰の値の物理的な正しさを示すものではない。
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
:math:`\dot{\bm{W}}` は、機体が風の高度分布の中を上下に動くことによる風の変化（風のシア）である。
機体が回転しなくても、風が高さとともに変われば横向きの対気速度は変わり、このモーメントが働く。
対流の回転の項の力

.. math::
   :label: eq-lag-conv-force

   \Delta\bm{F}_{b}=-\frac{\rho S}{2}\,L_0\,f_{\mathrm{lag}}(\alpha_t')\,(0,\,-\omega_zu_x,\,\omega_yu_x)

は角速度だけで決まるので、加速度の計算の前に加える。回転による法線力（:eq:`eq-cnq-force`）と合わせた :math:`\omega_y` の項の力は、細長い物体の理論の :math:`-\rho V\omega_yA_el_e` になる。
:math:`\dot{\bm{v}}` はこの力を含めた加速度である。遅れの項の力 :math:`-\tfrac12\rho SL_0\,\dot{\bm u}_\perp` は加えない（加えると加速度がこの力自身に依存し、代数的なループになる）。
その大きさは、機体の横向きの加速度による分では法線力の :math:`\tfrac12\rho SL_0/m` 倍（サンプルで 0.002 程度）、ピッチ振動の迎角の変化による分では法線力の振幅の :math:`L_0\omega_n/(C_{N\alpha}V)` 倍（サンプルで約 3 %、位相は 90° ずれる）である。
見かけの質量の反作用は流れの向きによらないので、後ろ向きの飛行でも向きを反転しない。

:math:`f_{\mathrm{lag}}` は、迎角の大きい流れでこの項を消す係数である。

.. math::
   :label: eq-lag-fade

   f_{\mathrm{lag}}(\alpha)=|\cos\alpha|\,\bigl(1-3s^2+2s^3\bigr),\qquad s=\min\!\left(1,\ \max\!\left(0,\ \frac{\alpha-20^\circ}{10^\circ}\right)\right)

20° までは :math:`\cos\alpha`、20° から 30° で滑らかに 0 になり、30° を超えると 0 である。
見かけの質量の理論は小さな迎角の理論で、迎角が大きいと横流れがはく離し、静的な係数（:eq:`eq-body-lift`）がその力を表すからである。

飛行経路の向きがほぼ一定のピッチ振動では :math:`\dot u_z\approx V\omega_y` で、どちらの項も角速度と同じ位相で効くので、\ :eq:`eq-lag-moment` はピッチ減衰を :math:`K` から :math:`K-2\,(x_{cg}L_0-L_1)` に変える（:math:`K` は\ :eq:`eq-moment-damp` の括弧の中）。
見かけの質量の中心 :math:`L_1/L_0` が重心より後ろなら減衰を強め、前なら弱める。
サンプル（:math:`M=0.3` で :math:`L_0=4.168` m、:math:`L_1=4.283` m\ :sup:`2`、中心 1.028 m。うちフィンが 1.445 m と 2.057 m\ :sup:`2`）では、
:math:`-2(x_{cg}L_0-L_1)/K` は燃焼前の重心 0.959 m で 0.19、燃焼後の重心 0.850 m で 0.40 で、局所的な迎角の和の減衰を 19 % と 40 % 強める。
ピッチ減衰の全体に対する見かけの質量の項の割合は 16 % と 29 % である。

回転による法線力
~~~~~~~~~~~~~~~~

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

重心まわりの減衰の和 :math:`S_2-2x_{cg}S_1+x_{cg}^2S_0` が負で 0 に打ち切られるとき（:eq:`eq-damping`）は、この力も加えない。
この力と対流の回転の力（:eq:`eq-lag-conv-force`）は、ピッチの固有振動数の 2 乗を :math:`1-c_q-c_c` 倍にする（:math:`c_q=\rho S(S_1-x_{cg}S_0)/(2m)`、:math:`c_c=\rho SL_0/(2m)`）。
サンプルの地上付近（:math:`M=0.3` の和）では、燃焼前と燃焼後で :math:`c_q` は 0.0010 と 0.0022、:math:`c_c` は 0.0018 と 0.0025 で、ほかの項より小さい。

Magnus 力
~~~~~~~~~

ロールしている機体が迎角を持つと、迎角の面に垂直な横力（Magnus 力）が生じる :cite:`mccoy`。

.. math::
   :label: eq-magnus

   \Delta\bm{F}_{b}=q\,S\,C_{Np\alpha}^{*}\,\hat p\,\frac{\bm{u}\times\bm{e}_x}{V},\qquad \hat p=\frac{\omega_xd}{2V},\qquad
   |\bm{u}\times\bm{e}_x|=V\sin\alpha_t

で、:math:`C_{Np\alpha}^{*}` は表の ``cnpa``\ [1/rad] である。
大きさは :math:`C_{N,\mathrm{Magnus}}=C_{Np\alpha}^{*}\,\hat p\,\sin\alpha_t` で、向き :math:`\bm{u}\times\bm{e}_x=(0,u_z,-u_y)` は静的な法線力の面から 90° 回った向きである。
:math:`\omega_x>0` で対気速度の横の成分が :math:`+z_b` 向き（:math:`u_z>0`）なら、``cnpa`` が正のとき力は :math:`+y_b` を向く。
McCoy の Magnus 力の係数 :math:`C_{Np\alpha}` は :math:`pd/V` で無次元化するので、:math:`\hat p=pd/(2V)` を使う表の値は :math:`C_{Np\alpha}^{*}=2C_{Np\alpha}` である。
符号の約束も同じである。McCoy の 6 自由度の式の Magnus の加速度は :math:`(\rho Sd\,p\,C_{Np\alpha}/2m)(\bm{v}\times\bm{x})`\ （:math:`\bm{v}` は機体の対気速度、:math:`\bm{x}` は機軸の単位ベクトル）:cite:`mccoy` で、
:math:`\bm{v}\times\bm{x}` は\ :eq:`eq-magnus` の :math:`\bm{u}\times\bm{e}_x` である。McCoy によれば、弾丸の :math:`C_{Np\alpha}` はふつう小さな負の値である。
Magnus 力は静的な法線力の圧力中心 :math:`x_{cp}`\ （一般表では :eq:`eq-xcp-eff` の等価な圧力中心）に働くとし、そのモーメント :math:`(x_{cg}-x_{cp},0,0)^\top\times\Delta\bm{F}_b` を加える。
これに、圧力中心まわりの Magnus のモーメント

.. math::
   :label: eq-magnus-moment

   \Delta\bm{M}_{b}=q\,S\,d\,C_{mp\alpha}^{*}\,\hat p\,\frac{(0,\,u_y,\,u_z)}{V}

を加える（:math:`C_{mp\alpha}^{*}` は表の ``cmpa``\ [1/rad]。既定は 0）。向きは横の対気速度の向きで、McCoy の Magnus のモーメント :math:`C_{Mp\alpha}` の向き :math:`\bm{v}-(\bm{v}\cdot\bm{x})\bm{x}` と同じである。
McCoy の重心まわりの係数との関係は :math:`2C_{Mp\alpha}=C_{Np\alpha}^{*}(x_{cg}-x_{cp})/d+C_{mp\alpha}^{*}` である。

Magnus の係数は、部品積み上げ法、パネル法、CFD のどれも推算しない。値は、回転させた機体の風洞試験（回転する支持装置）、ロール角速度を与えた CFD、実射の同定から求め、
:math:`pd/V` で無次元化した値なら 2 倍して外部の表に書く（:ref:`sec-file-axisym`）。フィン付きの機体では Magnus の力とモーメントの多くはフィンが作り、迎角と Mach 数で符号が変わることもあるので、迎角ごとの値を表にする。
表にこれらの値がないまま ``magnus_scale`` のばらつきを与えると、警告を出す（:ref:`sec-config-checks`）。

誘導ロール
~~~~~~~~~~

十字形のフィンでは、横流れがフィンの面から外れた向きから当たると、迎角の 2 乗に比例するロールのモーメントが生じる。
軸対称表では、ロールの係数（:eq:`eq-cl-axisym`）に

.. math::
   :label: eq-induced-roll

   \Delta C_l=C_{l4}\,\alpha_t'^{\,2}\,\sin\bigl(4(\phi_c-\phi_f)\bigr)\,\sgn(u_x),\qquad \phi_c=\operatorname{atan2}(u_z,\,u_y)

を加える（:math:`C_{l4}` は表の ``cl4``、:math:`\alpha_t'` は rad）。
:math:`\phi_f` は付随ファイルの ``fin_roll_deg``\ （1 枚のフィンのロール角。:math:`+y_b` から :math:`+z_b` へ測る）である。
横流れがフィンの面にあるとき（:math:`+` 形と :math:`\times` 形）は対称なので 0 で、22.5° ずれたとき最大になる。カントの強制と同じく、後ろ向きの飛行では向きを反転する。
:math:`\sin4\phi` と :math:`\alpha^2` の形は十字形（4 枚）のフィンのもので、付随ファイルの ``fin_count`` が 4 でない表では使わない（記録がない表は 4 枚とみなす。部品積み上げ法、パネル法、CFD の表には形状のフィンの枚数を書く）。
一般表では、展開のときにこの項を :math:`C_l` の列に入れる（:ref:`sec-aero-general`）。

.. _sec-roll-resonance:

ロールとピッチの共振の確認
~~~~~~~~~~~~~~~~~~~~~~~~~~

機体に固定した向きの釣り合いの強制（トリムの強制）があると、その強制は機体とともにロール角速度 :math:`p` で回る。
:math:`p` がピッチ・ヨーの振動の固有振動数に等しくなると、強制がその振動を駆動し、迎角が大きくなる（ロール共振。ロール角速度がそこにとどまると、ロックインや破壊的な首振りになる）。
飛翔計算は、次のトリムの強制が 1 つでもあるとき、ロール角速度が共振のロール角速度を横切った時刻を記録する。
対称なフィンのカントは機体をロールさせるだけで、迎角の向きを機体に固定する非対称を作らないので、トリムの強制には数えない（ロールの原因としてだけ働く）。

* 乾燥重心の横ずれ（``cg_lateral_dry`` が 0 でない。モンテカルロ解析のばらつきを含む）
* 推力線の傾きまたはノズルのオフセット
* 迎角 0、横滑り角 0 のモーメント :math:`|C_m|` または :math:`|C_n|` が :math:`10^{-4}` を超える一般表（非対称な機体の空力的なトリム）

ピッチの固有振動数は

.. math::
   :label: eq-roll-resonance

   \omega_n=\sqrt{\frac{\max(0,\,R)\,q\,S\,d}{\bar I}},\qquad \bar I=\frac{I_{yy}+I_{zz}}{2},\qquad
   p_r=\frac{\omega_n}{\sqrt{1-I_{xx}/\bar I}}

で、:math:`R` は静安定余裕の判定と同じ小迎角の復元の微係数（:ref:`sec-static-margin-general`。``cn_scale`` の倍率を含む）、:math:`\bar I` はそのときの横の慣性モーメントの平均である。
ロールしている機体の速いモードの振動数は :math:`\omega_f=\tfrac12\bigl(P+\sqrt{P^2+4\omega_n^2}\bigr)`\ （:math:`P=(I_{xx}/\bar I)\,p`。:ref:`sec-verify-unsteady` の Murphy の理論）で、ジャイロの効果で :math:`\omega_n` より大きい。
機体とともに :math:`p` で回る強制がこのモードに一致する :math:`p=\omega_f` を解いたものが共振のロール角速度 :math:`p_r` である（サンプルでは :math:`I_{xx}/\bar I\approx0.013` で、:math:`p_r` は :math:`\omega_n` より 0.7 % 大きい）。
自由飛行のうち頂点より前で対気速度が 10 m/s を超える間に、ロール角速度の大きさ :math:`|\omega_x|` が :math:`p_r` を横切った最初の時刻とその Mach 数、ロール角速度、交差の回数を記録する。結果は ``summary.json`` の ``roll_resonance``\ （:ref:`sec-file-summary`）に書き、警告を出す（:ref:`sec-config-checks`）。
トリムの強制がない機体（サンプルの既定の設定と、カントだけを付けた機体）では確かめない。

例えば、サンプルのフィンに 1° のカントを付け、乾燥重心を 1 mm 横にずらすと、ロール角速度は上昇の初めに共振のロール角速度を横切り、
``warning[IY-O510]: roll resonance: the roll rate crosses the resonance rate of the pitch oscillation (10.2 rad/s) at t = 0.61 s, Mach 0.36; the lateral CG offset, thrust misalignment or asymmetric aero moment can amplify the angle of attack there``
の警告が出る。このあとロール角速度は約 88 rad/s まで上がり（:ref:`sec-vaero-barrowman-roll-flight`）、交差は 1 回である。
この確認は交差を知らせるだけで、迎角の増え方の大きさは飛翔計算そのもの（``trajectory.csv`` の ``alpha_deg``）で見る。

.. _sec-quasi-steady:

準定常の仮定
~~~~~~~~~~~~

この節の項は、どれもそのときの状態と加速度だけで決まり、流れの履歴を表す状態量（遅れの状態や動的失速の状態）を持たない。
その根拠は、機体の振動が流れの時間尺度に比べて遅いことである。
フィンの平均空力翼弦 :math:`c=0.155` m を使った減次振動数 :math:`k=\omega_nc/(2V)` は、サンプルの上昇中（対気速度 10 m/s 超）で 0.006〜0.010、中央値 0.007 である。
流れがフィンの翼弦を通り過ぎる時間 :math:`c/V` は、振動の周期の :math:`k/\pi\approx0.2\,\%` にあたる。
2 次元の翼の Theodorsen の関数 :math:`C(k)` :cite:`theodorsen` は、:math:`k=0.007` で :math:`0.988-0.035\,i` で、揚力の振幅を 1.1 % 下げ、位相を 2.0°（周期の 0.6 %）遅らせる。
ただし 2 次元の翼の位相の遅れは、:math:`k\to0` で :math:`k\ln(1/k)` のように残る後流（翼幅が無限の渦の後流）によるもので、そのまま使うとフィンの減衰と同じ程度の大きさを見積もってしまう。
アスペクト比の小さいフィンでは、翼端から後流の渦が閉じるので、この遅れはずっと小さい。

R. T. Jones の有限翼の指示関数 :cite:`jones1940` は、迎角を急に変えたときの揚力の立ち上がりを :math:`\varphi(s)=1-a\,e^{-bs}`\ （:math:`s` は半翼弦を単位に測った進んだ距離）で表し、
アスペクト比 6 で :math:`a=0.361`、:math:`b=0.381`、アスペクト比 3 で :math:`a=0.283`、:math:`b=0.540` である。
ゆっくりした運動では、揚力は迎角の変化に時間 :math:`\tau=(a/b)\,c/(2V)` だけ遅れ、:math:`a/b` はアスペクト比 6 で 0.95、3 で 0.52 で、アスペクト比が小さいほど小さくなる（細長い物体の理論の極限では 0）。
サンプルのフィン（2 枚の組のアスペクト比 1.33）では、この 2 点からの外挿で :math:`a/b\approx0.25` とする。
この値は外挿で、不確かさが大きい。胴体はフィンの付け根で一部が反射面として働くので、実効的なアスペクト比は 1.33 より大きく（胴体の直径を含めると約 1.8）、:math:`a/b` は 0.1〜0.4 程度の幅で見る。
フィンの揚力が :math:`\tau` 遅れると、フィンの減衰のモーメントは :math:`\tau V/l_f=(a/b)(c/2)/l_f` の割合で弱まる（:math:`l_f` はフィンの圧力中心の重心からの距離。重心 0.959 m と 0.850 m で 0.415 m と 0.524 m）。
:math:`a/b=0.25` でフィンの減衰の 3.7〜4.7 %、:math:`a/b` の幅を入れると 2〜8 % である。フィンの局所的な迎角の和はサンプルのピッチ減衰の全体の 46〜52 % なので、全体では約 2 %（幅を入れて 1〜4 %）である。
揚力の大きさの変化は 1 % 程度で、法線力の不確かさ（``cn_scale``）よりずっと小さい。

この遅れはモデルに入れず、ピッチ減衰の不確かさとして見込む。
危険区域を求めるモンテカルロ解析では、``cmq_scale``\ （局所的な迎角の和）の標準偏差を 0.1、``lag_scale``\ （見かけの質量の項）の標準偏差を 0.25 にすることを勧める（:ref:`sec-mc`）。
``cmq_scale`` は局所的な迎角の和だけに掛かり、ピッチ減衰の 16〜29 %（サンプル）を占める見かけの質量の項には掛からないので、両方を与える。
``lag_scale`` の 0.25 は、フィンの見かけの質量を Barrowman の傾きにそろえる仮定（細長い物体の理論のままなら 1.47 倍）の幅を見込む大きさである。
ただし、循環の遅れ（減衰を約 2 % 弱める）と、細長い物体の理論の閉じた式に対するモデルの差（サンプルで 6〜7 % 強い）は、どちらも向きの決まった偏りである。
平均 0 の正規分布のばらつきは偏りを表さないので、減衰が弱い側の見積もりが要るときは、減衰の和を 0.9 倍にした外部の表（:ref:`sec-file-v1`）で別に計算する。
既定は、どちらも 0 である（偏りを加えない）。
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

計算例
~~~~~~

付属のサンプル（射角 85°、方位 270°、北風 4 m/s、パラシュートあり）の結果を :numref:`fig-trajectory` と :numref:`fig-flight-states` に示す。

.. _fig-trajectory:

.. figure:: /_generated/plots/trajectory.*
   :width: 100%

   高度の時刻歴（左）と地表への投影（右）。見るところは、頂点の時刻と高度、降下に入って高度が緩やかに下がる区間、着地点が風下へどれだけ流されるかである。

.. _fig-flight-states:

.. figure:: /_generated/plots/flight_states.*
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
     - 518.8 m/s / 1.54
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

適用範囲と限界
--------------

全体の一覧は :ref:`sec-lim-motion` にある。この章のモデルに関わる境界は次のとおりである。

* 機体は剛体で、質量の時間変化による慣性テンソルの微分 :math:`\dot I` は無視する。推進剤は軸上の一様な円柱とする。
* 空気力は 6 成分の準定常な係数、対角の回転減衰、回転による法線力、見かけの質量の項、Magnus 力、誘導ロールで表す。
  動的失速、大迎角でのはく離の遅れ、フィンの揚力の循環の遅れは扱わない。
  循環の遅れは、サンプルのフィンでフィンの減衰を約 4 %、ピッチ減衰の全体を約 2 % 弱めると見積もった（:ref:`sec-quasi-steady`）。向きの決まった偏りなので、平均 0 のばらつきでは表せない。
* 軸対称な機体では、ある軸の角速度が別の軸のモーメントを作る線形の交差微係数は 0 である。非軸対称な形状でこれらが 0 でない機体は、外部の一般表でも表せない。
* 静的な安定は、迎角 0 の極限の小迎角の余裕の符号で判定する。大きな迎角での釣り合いと、ピッチとヨーの連成は調べない。
  一般表の最初の格子点は 2° で、軸対称表の 1° より広いので、同じ機体の余裕はサンプルで約 0.1 cal 小さく出る。
* ロール共振は、ロール角速度が共振の値を横切ったことを知らせるだけで、迎角の増え方は飛翔の結果（``trajectory.csv`` の ``alpha_deg``）で見る。

精度の根拠
----------

次の比較は、モデルどうし、または解析解との検証である。実測の飛翔との比較（妥当性確認）はない。

* 見かけの質量の項を含むピッチ減衰は、細長い物体の理論の閉じた式（:eq:`eq-sbt-damping`）と、サンプルで 6〜7 % 以内で一致する。
  閉じた式の :math:`C_{N\alpha}` もフィンの見かけの質量も Barrowman の傾きによるので、この一致は項の組み立ての整合性の検証であり、減衰の値の物理的な正しさを示すものではない（:ref:`sec-verify-unsteady`）。
* 軸対称表の最小静安定余裕は、各時刻の Mach 数と重心で求めた小迎角の余裕の最小値と 0.02 cal 以内で一致する（:ref:`sec-verify-margin`）。
* ステップ内の事象と二段開傘の扱いは、解析解のある条件で検証している（:ref:`sec-verify-events`、:ref:`sec-verify-dual`）。
* 空力係数そのものは、Basic Finner の弾道射場データ（Mach 1.08〜4.30）などと比べている（:ref:`sec-validation`）。

関連する設定キー
----------------

.. list-table::
   :header-rows: 1
   :widths: 34 16 50

   * - キー
     - 既定値
     - 効果の方向
   * - :ref:`sim.quasi_steady <cfg-sim-quasi_steady>`
     - ``false``
     - ``true`` で見かけの質量の項、回転による法線力、Magnus の項を切る。着地点は 1 m 程度しか変わらず、上昇中の最大迎角が約 8 % 小さくなる（:numref:`tbl-quasi-steady-effect`）。
   * - :ref:`aero.table_kind <cfg-aero-table_kind>`
     - ``"axisymmetric"``
     - ``"general"`` で 6 成分の係数を使う。静安定余裕の最初の格子点が粗くなる（上の「適用範囲と限界」）。
   * - :ref:`rocket.cg_lateral_dry <cfg-rocket-cg_lateral_dry>`
     - ``[0, 0]``
     - 0 でないと機体は軸対称でなくなり、ロール共振の確認が働く。
   * - :ref:`motor.misalignment_deg <cfg-motor-misalignment_deg>`
     - 0
     - 推力の傾きが横力と機首を振るモーメントを作る。
   * - :ref:`lag_scale <cfg-dispersion-monte_carlo-lag_scale>`、:ref:`magnus_scale <cfg-dispersion-monte_carlo-magnus_scale>`、:ref:`cmq_scale <cfg-dispersion-monte_carlo-cmq_scale>`
     - 0
     - 非定常な項と減衰の不確かさを、標準偏差の分だけ倍率として与える。
