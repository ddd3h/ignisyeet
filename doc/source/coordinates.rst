座標系と姿勢
============

IgnisYeet では次の四つの座標系を使う。

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - 座標系
     - 用途
   * - 局所 ENU 座標系
     - 平面地球モードでは運動方程式を積分する慣性系（とみなす座標系）。ECEF モードでは出力の表示用

   * - 機体座標系
     - 推力・空気力・モーメントを表す座標系
   * - ECEF 座標系
     - ENU と緯度経度を結ぶ中間の座標系。ECEF モードでは運動方程式を積分する回転座標系
   * - 測地座標系（LLH）
     - 射点と着地点を緯度・経度・高度で入出力する

局所 ENU 座標系
---------------

射点を原点とし、:math:`x_E` 軸を東、:math:`y_N` 軸を北、:math:`z_U` 軸を天頂（射点での楕円体の法線方向）にとる右手系である :cite:`torge`\ （:numref:`fig-frames`）。位置ベクトルを :math:`\bm{x}=(x_E,\,y_N,\,z_U)^\top` と書く。
:math:`z_U` は **射点からの高さ** であり、海抜高度は :math:`h_0+z_U`\ （:math:`h_0` は射点標高）である。

既定の平面地球モード（``earth.model = "flat"``）では、IgnisYeet はこの座標系を **慣性系とみなし**、地表を平面として扱う（平面地球近似）。
地球の自転（Coriolis 力・遠心力）と地表の曲率は無視する。
この近似がどこまで許されるかは :ref:`sec-enu-range` で見積もる。
これらを考慮したいときは、ECEF 座標系で積分する ECEF モード（``earth.model = "ecef"``）を使う（:ref:`sec-ecef`）。
ECEF モードでも、位置・速度などの **出力** は常に射点の ENU 座標系で表す。

.. _fig-frames:

.. figure:: _generated/tikz/frames.*
   :width: 80%

   局所 ENU 座標系と、ランチャの方位角 :math:`\psi`・射角 :math:`\theta_e`。

機体座標系
----------

重心を原点とし、:math:`x_b` 軸を機軸に沿って **ノーズ方向**、:math:`y_b`, :math:`z_b` 軸をそれに直交するようにとる
（:numref:`fig-body-frame`）。機体は軸対称と仮定するので、:math:`y_b`, :math:`z_b` 軸の向きは任意である。

一方、機体上の **位置** （重心 :math:`x_{cg}`、圧力中心 :math:`x_{cp}`、フィン位置など）は、
直感的に分かりやすいように **ノーズ先端から後方へ測った距離** :math:`x` で表す。
したがって機体上の点 :math:`x` の機体座標は :math:`(x_{cg}-x,\,0,\,0)` である。

.. _fig-body-frame:

.. figure:: _generated/tikz/body_frame.*
   :width: 85%

   機体座標系と機体に働く力。迎角 :math:`\alpha` は機軸と対気速度 :math:`\bm{v}_a` のなす角である。

姿勢の表現（クォータニオン）
----------------------------

機体座標系から ENU 座標系への回転を単位クォータニオン :math:`\bm{q}=(q_0,q_1,q_2,q_3)` で表す :cite:`shuster`。
対応する回転行列は

.. math::
   :label: eq-quat-mat

   R(\bm{q}) =
   \begin{pmatrix}
   1-2(q_2^2+q_3^2) & 2(q_1q_2-q_0q_3) & 2(q_1q_3+q_0q_2)\\
   2(q_1q_2+q_0q_3) & 1-2(q_1^2+q_3^2) & 2(q_2q_3-q_0q_1)\\
   2(q_1q_3-q_0q_2) & 2(q_2q_3+q_0q_1) & 1-2(q_1^2+q_2^2)
   \end{pmatrix}

で、機体座標で表したベクトル :math:`\bm{a}_b` は :math:`\bm{a}=R\,\bm{a}_b` で ENU 座標に、
逆に :math:`\bm{a}_b=R^\top\bm{a}` で機体座標に変換できる。:math:`R` の各列は機体の各軸を ENU で表したものである。

機体座標系での角速度を :math:`\bm{\omega}=(\omega_x,\omega_y,\omega_z)` とすると、クォータニオンの時間変化は :cite:`shuster`

.. math::
   :label: eq-quat-dot

   \dot{\bm{q}} = \frac12\,\bm{q}\otimes(0,\bm{\omega})
   = \frac12
   \begin{pmatrix}
   -q_1\omega_x-q_2\omega_y-q_3\omega_z\\
   \phantom{-}q_0\omega_x+q_2\omega_z-q_3\omega_y\\
   \phantom{-}q_0\omega_y-q_1\omega_z+q_3\omega_x\\
   \phantom{-}q_0\omega_z+q_1\omega_y-q_2\omega_x
   \end{pmatrix}

である。オイラー角と違って特異点（ジンバルロック）がなく :cite:`shuster`、機体が真上を向く打上げ直後でも安定に計算できる。
数値積分では :math:`|\bm{q}|=1` が少しずつ崩れるので、各ステップの後で正規化する。

初期姿勢
~~~~~~~~

ランチャ上の機軸方向は、射角 :math:`\theta_e` と方位角 :math:`\psi`\ （北から時計回り）から

.. math::
   :label: eq-rail-dir

   \bm{d} = \left(\cos\theta_e\sin\psi,\ \cos\theta_e\cos\psi,\ \sin\theta_e\right)^\top

である。機体軸を :math:`\bm{e}_{x_b}=\bm{d}`、:math:`\bm{e}_{y_b}=(\cos\psi,\,-\sin\psi,\,0)^\top`\ （水平で :math:`\bm{d}` に直交）、
:math:`\bm{e}_{z_b}=\bm{e}_{x_b}\times\bm{e}_{y_b}` とし、これらを列に並べた回転行列からクォータニオンを求めて初期値とする。

地球楕円体と測地座標
--------------------

射点と着地点を緯度・経度で扱うため、地球を WGS84 楕円体 :cite:`wgs84,torge` で近似する（:numref:`fig-ellipsoid`）。

.. list-table:: WGS84 楕円体の定数
   :header-rows: 1
   :widths: 40 60

   * - 量
     - 値
   * - 赤道半径 :math:`a`
     - 6 378 137.0 m
   * - 扁平率 :math:`f`
     - 1 / 298.257 223 563
   * - 極半径 :math:`b=a(1-f)`
     - 6 356 752.314 m
   * - 第一離心率の 2 乗 :math:`e^2=f(2-f)`
     - 0.006 694 380 0
   * - 第二離心率の 2 乗 :math:`e'^2=(a^2-b^2)/b^2`
     - 0.006 739 496 7

.. _fig-ellipsoid:

.. figure:: _generated/tikz/ellipsoid.*
   :width: 65%

   楕円体の子午線断面。測地緯度 :math:`\phi` は楕円体面の法線と赤道面のなす角、
   :math:`N(\phi)` は法線に沿って自転軸までの長さ（卯酉線曲率半径）である。

測地座標から ECEF 座標へ
~~~~~~~~~~~~~~~~~~~~~~~~

緯度 :math:`\phi`、経度 :math:`\lambda`、楕円体高 :math:`h` から ECEF 座標 :math:`(X,Y,Z)` へは、次の式で厳密に変換できる :cite:`torge,hofmann`。

.. math::
   :label: eq-lla-ecef

   N(\phi) = \frac{a}{\sqrt{1-e^2\sin^2\phi}},\qquad
   \begin{aligned}
   X &= (N+h)\cos\phi\cos\lambda\\
   Y &= (N+h)\cos\phi\sin\lambda\\
   Z &= \bigl(N(1-e^2)+h\bigr)\sin\phi
   \end{aligned}

:math:`N` は卯酉線（東西方向）の曲率半径で、:numref:`fig-ellipsoid` のように法線に沿って測った自転軸までの距離に等しい。

ECEF 座標から測地座標へ
~~~~~~~~~~~~~~~~~~~~~~~

逆変換は閉じた形で書けないが、Bowring の方法 :cite:`bowring` を用いると 1 回の計算で十分な精度が得られる。

.. math::
   :label: eq-ecef-lla

   \begin{aligned}
   \lambda &= \operatorname{atan2}(Y,\,X), \qquad p=\sqrt{X^2+Y^2},\qquad
   \vartheta = \operatorname{atan2}(Z\,a,\ p\,b),\\
   \phi &= \operatorname{atan2}\!\left(Z+e'^2\,b\sin^3\vartheta,\ \ p-e^2a\cos^3\vartheta\right),\\
   h &= \frac{p}{\cos\phi}-N(\phi).
   \end{aligned}

:ref:`sec-verify-geo` で示すように、高度 100 km までの往復変換の誤差は 0.1 mm 未満であり、反復は不要である。

ENU 座標から測地座標へ
~~~~~~~~~~~~~~~~~~~~~~

射点 :math:`(\phi_0,\lambda_0,h_0)` を ECEF に直した点を :math:`\bm{X}_0` とする。
射点での東・北・天頂の単位ベクトルを ECEF で表すと

.. math::
   :label: eq-enu-basis

   \bm{e}_E=\begin{pmatrix}-\sin\lambda_0\\ \cos\lambda_0\\ 0\end{pmatrix},\quad
   \bm{e}_N=\begin{pmatrix}-\sin\phi_0\cos\lambda_0\\ -\sin\phi_0\sin\lambda_0\\ \cos\phi_0\end{pmatrix},\quad
   \bm{e}_U=\begin{pmatrix}\cos\phi_0\cos\lambda_0\\ \cos\phi_0\sin\lambda_0\\ \sin\phi_0\end{pmatrix}

であり、ENU 座標 :math:`(x_E,y_N,z_U)` の点の ECEF 座標は
:math:`\bm{X}=\bm{X}_0+x_E\bm{e}_E+y_N\bm{e}_N+z_U\bm{e}_U` となる。
これに\ :eq:`eq-ecef-lla` を適用して緯度・経度を得る。
逆向き（ECEF から ENU）は、この三つのベクトルを行に並べた行列を :math:`\bm{X}-\bm{X}_0` に掛ければよい。

.. _sec-enu-range:

平面地球近似が使える範囲
------------------------

ENU 座標系は射点での接平面を基準にしているので、射点から離れるほど実際の地表（球面）とのずれが大きくなる。
地球を半径 :math:`R` の球とし、射点 :math:`O` から地表に沿って距離 :math:`d` 離れた点 :math:`P` を考える（:numref:`fig-enu-curvature`）。

.. _fig-enu-curvature:

.. figure:: _generated/tikz/enu_curvature.*
   :width: 60%

   接平面（ENU の水平面）と球面のずれ :math:`\Delta h`。

中心角を :math:`\vartheta=d/R` とすると、接平面から球面までの鉛直方向のずれは

.. math::
   :label: eq-enu-dh

   \Delta h = R\,(1-\cos\vartheta) \simeq \frac{R\vartheta^2}{2} = \frac{d^2}{2R}\qquad(d\ll R)

である。許容誤差を :math:`\varepsilon` とすると、平面近似が使える距離は

.. math::
   :label: eq-enu-dmax

   d \le \sqrt{2R\,\varepsilon}

となる。:math:`R=6371\ \mathrm{km}` として、:math:`\varepsilon=1\ \mathrm{m}` なら :math:`d\le 3.6\ \mathrm{km}`、
:math:`\varepsilon=10\ \mathrm{m}` なら :math:`d\le 11.3\ \mathrm{km}` である（:numref:`fig-enu-error`）。

.. _fig-enu-error:

.. figure:: _generated/plots/enu_error.*
   :width: 80%

   射点からの距離と接平面からのずれ :math:`\Delta h=d^2/2R`。

平面地球モードでは、着地の判定を :math:`z_U\le0`\ （接平面との交点）で行う。
着地点が射点から 10 km 離れると、実際の地表はそこからさらに約 8 m 低い。
機体はその分だけ長く落下するはずなので、計算上の着地点は実際より少し射点寄りになる。
落下角が浅いパラシュート降下ほど、この水平方向の誤差は大きくなる。
数 km 以内に落下する一般的なロケットでは無視できるが、高高度・長距離の飛翔では次節の ECEF モードを使うとよい。

.. _sec-ecef:

ECEF 座標での積分
-----------------

``earth.model = "ecef"`` とすると、運動方程式を WGS84 楕円体に固定した **回転座標系**\ （ECEF 座標系）で積分する。
平面地球近似と違い、地表の曲率（鉛直方向が射点から離れるにつれて変わること）、Coriolis 力、遠心力、扁平を考慮した重力（``gravity = "j2"``）を扱える。
機体の質量・空力・推力の扱いは平面地球モードと同じで、座標系に依存する部分だけが異なる。

状態量と加速度
~~~~~~~~~~~~~~

状態量は、ECEF 座標での位置 :math:`\bm{r}=(X,Y,Z)^\top` と **地球に対する** 速度 :math:`\bm{v}`、
機体座標から ECEF 座標への回転を表すクォータニオン :math:`\bm{q}`、
機体座標で表した **ECEF に対する** 角速度 :math:`\bm{\omega}`\ （13 個）である。
ECEF 座標系は地球の自転ベクトル :math:`\bm{\Omega}=(0,0,\Omega_\oplus)^\top`\ （:math:`\Omega_\oplus=7.292\,115\times10^{-5}\ \mathrm{rad/s}` :cite:`wgs84,iers`）で回転しているので、
重心の加速度は

.. math::
   :label: eq-ecef-accel

   \dot{\bm{r}}=\bm{v},\qquad
   \dot{\bm{v}} = \frac{1}{m}\,R(\bm{q})\left(\bm{F}_T+\bm{F}_A\right)_b + \bm{g}(\bm{r})
   - 2\,\bm{\Omega}\times\bm{v} - \bm{\Omega}\times(\bm{\Omega}\times\bm{r})

となる。右辺の第 1 項が推力と空気力、第 2 項が重力（遠心力を含まない万有引力のみ。:ref:`sec-gravity`）、第 3 項が Coriolis 加速度、第 4 項が遠心加速度である :cite:`goldstein`\ （:numref:`fig-ecef-rotating`）。ランチャ上の運動（:eq:`eq-rail`）も同じ加速度を使う。

.. _fig-ecef-rotating:

.. figure:: _generated/tikz/ecef_rotating.*
   :width: 60%

   自転する ECEF 座標系と射点の局所座標。上昇する機体には Coriolis 加速度が西向きに、
   自転軸から離れる向きに遠心加速度が働く。

北半球で上昇する機体の Coriolis 加速度の東西成分は :math:`-2\Omega_\oplus\cos\phi\,v_U` で、常に西向きである。
このため鉛直に近く打ち上げた機体は少し西にずれる（:ref:`sec-verify-ecef`）。

姿勢の方程式
~~~~~~~~~~~~

Euler の運動方程式（:eq:`eq-euler`）の角速度は **慣性系に対する** ものでなければならない :cite:`goldstein`。
ECEF に対する角速度 :math:`\bm{\omega}` に、自転ベクトルを機体座標で表した :math:`R^\top\bm{\Omega}` を足して

.. math::
   :label: eq-omega-inertial

   \bm{\omega}_I = \bm{\omega} + R^\top\bm{\Omega}

とし、

.. math::
   :label: eq-ecef-euler

   I\,\dot{\bm{\omega}}_I = \bm{M}_b - \bm{\omega}_I\times\left(I\,\bm{\omega}_I\right),\qquad
   \dot{\bm{\omega}} = \dot{\bm{\omega}}_I + \bm{\omega}\times\left(R^\top\bm{\Omega}\right)

で :math:`\bm{\omega}` の変化を求める。第 2 式は、:math:`\bm{\Omega}` が ECEF で一定なので
:math:`\mathrm{d}(R^\top\bm{\Omega})/\mathrm{d}t=-\bm{\omega}\times(R^\top\bm{\Omega})` となることから得られる。
クォータニオンは ECEF に対する角速度で更新する（:math:`\dot{\bm{q}}=\tfrac12\,\bm{q}\otimes(0,\bm{\omega})`）。
空力モーメントとピッチ減衰（:eq:`eq-moment-damp`）は :math:`\bm{\omega}` の横成分に対して計算する。
平面地球モードでは :math:`\bm{\Omega}=0` なので、これらは\ :eq:`eq-euler` に一致する。

初期状態
~~~~~~~~

射点の測地座標 :math:`(\phi_0,\lambda_0,h_0)` から\ :eq:`eq-lla-ecef` で :math:`\bm{r}_0` を求め、
:math:`\bm{v}=0`\ （機体は地球とともに回転しているので、地球に対して静止）、:math:`\bm{\omega}=0` で始める。
機体の初期姿勢は、ランチャの方向（:eq:`eq-rail-dir`）と機体軸の定義を射点の ENU 基底（:eq:`eq-enu-basis`）で ECEF 座標に直して作る。

現在位置での局所座標
~~~~~~~~~~~~~~~~~~~~

積分の各ステップで、現在の ECEF 位置から\ :eq:`eq-ecef-lla` で測地座標 :math:`(\phi,\lambda,h)` を求め、次の量を決める。

* **高さ** :math:`z=h-h_0`\ （射点標高からの楕円体高の差）。大気・音速は海抜高度 :math:`h` で、重力は\ :eq:`eq-gravity` の :math:`h_0+z` で評価する。
* **局所の東・北・天頂** :math:`\bm{e}_E,\bm{e}_N,\bm{e}_U`\ （:eq:`eq-enu-basis` を現在の :math:`(\phi,\lambda)` で評価する）。
* **風**：高度 :math:`z` での風速（:ref:`sec-wind`）の水平成分を :math:`\bm{w}=w_E\bm{e}_E+w_N\bm{e}_N` として ECEF 座標に直し、
  対気速度を :math:`\bm{v}_a=\bm{v}-\bm{w}` とする。鉛直成分は 0 である。
* **重力**：``inverse_square`` と ``constant`` では、大きさは平面地球モードと同じで（それぞれ :math:`g_0(R_\oplus/(R_\oplus+h_0+z))^2` と :math:`g_0`）、
  向きを現在位置の測地的な下向き :math:`-\bm{e}_U` にとる。``j2`` では :ref:`sec-gravity` の\ :eq:`eq-j2` を ECEF 位置で評価した
  ベクトルをそのまま使う（水平成分を含む）。

着地と出力
~~~~~~~~~~

着地の判定は、ランチャ離脱後に **測地的な高さ** :math:`z=h-h_0` が 0 以下になったステップで行う。
平面地球モードでは接平面との交点だったのに対し、こちらは曲率を含めた射点標高の楕円体面との交点である。
直前のステップとの間の高さを線形補間して、着地時刻と位置を求める。
頂点の判定は、速度の天頂成分 :math:`\bm{v}\cdot\bm{e}_U`\ （現在位置の天頂）の符号の変化で行う。

出力（``trajectory.csv`` と ``summary.json``）は、すべて **射点の ENU 座標系** で表す。
位置は :math:`(\bm{r}-\bm{r}_0)` を射点の :math:`\bm{e}_E,\bm{e}_N,\bm{e}_U` に射影した接平面座標（東・北・上）、
速度は同じ基底への射影（地球に対する速度）である。
したがって ECEF モードでの「上」は射点の接平面からの高さであり、遠方では楕円体高 ``alt`` と一致しない。
頂点高度 ``apogee`` は測地的な高さ :math:`z` である。
着地点の東・北と着地距離もこの接平面座標、緯度・経度は ECEF 位置から求めた測地座標である。
ピッチ角と方位角は、現在位置の局所の天頂・東・北に対して測る。

平面地球モードとの比較
~~~~~~~~~~~~~~~~~~~~~~

サンプル（パラシュート降下、RK45）を同じ条件で計算した結果を :numref:`tbl-ecef-compare` に示す。
重力モデルは、平面地球が逆 2 乗則、ECEF が逆 2 乗則と :math:`J_2` である。

.. _tbl-ecef-compare:

.. list-table:: 平面地球と ECEF の比較（サンプル、パラシュート降下）
   :header-rows: 1
   :widths: 34 16 16 16 18

   * - モード
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
     - 飛行時間 [s]
   * - 平面地球（逆 2 乗則）
     - 4440.7
     - −654.8
     - −7707.9
     - 685.0
   * - ECEF（逆 2 乗則）
     - 4444.3
     - −662.3
     - −7733.3
     - 686.3
   * - ECEF（:math:`J_2`）
     - 4441.6
     - −661.9
     - −7710.5
     - 685.4

数 km 規模の飛翔では、着地点の差は 7.7 km の飛行距離に対して数十 m にとどまる。
風や空力係数の不確かさに比べて小さいので、この規模では平面地球モードで十分であり、
高高度・長距離の飛翔や、Coriolis 力そのものを評価したいときに ECEF モードを使えばよい。
