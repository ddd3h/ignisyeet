.. _page-theory-integrators:

数値積分
========

運動方程式（:ref:`page-theory-dynamics`）の状態方程式 :math:`\dot{\bm{y}}=\bm{f}(t,\bm{y})` は、数値積分で解く。
積分法には、固定刻みの ``rk4``、刻み幅を誤差で制御する ``rk45`` と ``dop853`` の三つがあり、``sim.integrator`` で選ぶ。
これとは別に、姿勢（クォータニオン）の更新法として、正規化する ``normalize`` と Lie 群の上で直接進める ``lie_group`` を ``sim.attitude`` で選ぶ。

どの積分法でも、サンプルの頂点と着地点は 0.01 m 以内で一致する。
選択が結果に効くのは、多数の飛翔を速く計算するときと、ロール角速度が大きく ``rk4`` の 1 ステップの回転角が大きくなるときである。
ランチャ離脱・頂点・開傘・着地の時刻は、ステップの中の補間で求めるので、刻み幅にほとんど依存しない。

.. _sec-integrators:

積分の進め方
------------

どの積分法でも、1 ステップの後で（``attitude = "normalize"`` ではクォータニオンを正規化したうえで）、段階の切り替え（ランチャ離脱・頂点・開傘・着地）を判定する。
状態量に有限でない値が現れた飛翔や、速さが 8 km/s を超えるか高さが 1000 km を超えた飛翔は、数値的に発散したものとして ``IY-S081`` のエラーで終わる（:ref:`IY-S081`）。

.. _sec-step-events:

ステップ内の事象と刻みの区切り
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ランチャ離脱、頂点、メインの開傘、着地は、刻みの途中で起こる。これらの時刻と状態は、ステップの中の補間で求めるので、刻み幅にほとんど依存しない。
開傘の時刻は、頂点の時刻に ``delay`` を足した時刻ちょうどである。
刻み幅 :math:`h` のステップの両端の位置 :math:`\bm{x}_0,\bm{x}_1` と速度 :math:`\bm{v}_0,\bm{v}_1` から、位置の三次エルミート補間

.. math::
   :label: eq-step-hermite

   \bm{x}(s)=(2s^3-3s^2+1)\,\bm{x}_0+(s^3-2s^2+s)\,h\bm{v}_0+(-2s^3+3s^2)\,\bm{x}_1+(s^3-s^2)\,h\bm{v}_1,\qquad s\in[0,1]

を作る。事象の状態は、位置をこの補間、速度をその時間微分

.. math::
   :label: eq-step-hermite-vel

   \bm{v}(s)=\frac{1}{h}\frac{\mathrm{d}\bm{x}}{\mathrm{d}s}=\frac{6(s^2-s)}{h}\,(\bm{x}_0-\bm{x}_1)+(3s^2-4s+1)\,\bm{v}_0+(3s^2-2s)\,\bm{v}_1

で作り、姿勢と角速度などの残りは両端の線形補間で作る。
1 つのステップに複数の事象が入るときは、時刻の早い順に扱う。着地は同時刻の事象に優先し、着地より後の事象は起こらない。

刻みは次の時刻をまたがず、その時刻にちょうど合わせる。

* ``sim.max_time``：着地していなくても、この時刻に計算を終える。
* 推力曲線の折れ点（燃焼終了を含む）。
* パラシュート降下で開傘前なら、開傘の時刻。

各事象の扱いは :ref:`sec-rail`、:ref:`sec-parachute`、:ref:`sec-dual-deploy`、:ref:`sec-landing` にある。

頂点の時刻
~~~~~~~~~~

頂点の時刻は、どの積分法でも、:math:`v_z` が正から 0 以下に変わったステップの中で、そのステップの三次エルミート補間（:eq:`eq-step-hermite`）の鉛直速度

.. math::
   :label: eq-apogee-vz

   v_z(s)=\frac{1}{h}\,\frac{\mathrm{d}\bm{x}}{\mathrm{d}s}\cdot\bm{e}_U\bigl(\bm{x}(s)\bigr)

が 0 になる点 :math:`s_a` から、:math:`t_{\mathrm{apo}}=t-h+h\,s_a` とする（:math:`\bm{e}_U` はその位置での上向きの単位ベクトル。平面地球では :math:`z_U` の向き）。
頂点の高さと水平位置は、同じ補間を :math:`s_a` で評価した値で、時刻と位置は補間した高さの極大になる。
着地の位置と時刻は、三次エルミート補間した高さの零点で求める（:ref:`sec-landing`）。
``trajectory.csv`` には、時刻が ``output_interval`` の倍数以上になった最初のステップの端点を書き出す。補間はしないので、間隔は完全には一定でない。

古典的 Runge–Kutta 法（``rk4``）
--------------------------------

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

刻みは推力曲線の折れ点と燃焼終了の時刻に合わせて切り詰めるので、最後のステップは :math:`\Delta t` より短くなりうる。
1 ステップで右辺を 4 回評価する。

Dormand–Prince 法（``rk45``）
-----------------------------

刻み幅を誤差に応じて自動で調整する、埋め込み型の 5(4) 次 Runge–Kutta 法（Dormand–Prince 法 :cite:`dormand`）である :cite:`hairer`。
刻み幅 :math:`h` の 1 ステップは 7 段で、

.. math::
   :label: eq-dopri

   \bm{k}_i&=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\quad (i=2,\dots,6),\\
   \bm{y}_{n+1}&=\bm{y}_n+h\sum_{j=1}^{6}b_j\bm{k}_j,\qquad
   \bm{k}_7=\bm{f}(t+h,\ \bm{y}_{n+1})

とする。係数 :math:`c_i,a_{ij},b_j` は Dormand と Prince の Butcher 表 :cite:`dormand` のとおりである。
5 次の解と 4 次の解の差 :math:`\bm{e}=h\sum_{j=1}^{7}E_j\bm{k}_j` が誤差の推定になる。
終点の傾き :math:`\bm{k}_7` を次のステップの出発点に使うので（FSAL :cite:`hairer`）、1 ステップあたりの右辺の評価は 6 回である。

rk45 の誤差と刻み幅の制御
~~~~~~~~~~~~~~~~~~~~~~~~~

誤差の推定は、13 個の状態量すべて（位置・速度・クォータニオン・角速度。単位や大きさの重み付けはしない）を
相対許容誤差 ``rtol``\ （:math:`\varepsilon_r`）と絶対許容誤差 ``atol``\ （:math:`\varepsilon_a`）で規格化し、二乗平均平方根をとる :cite:`hairer`。

.. math::
   :label: eq-dopri-err

   \mathrm{err}=\sqrt{\frac1{13}\sum_{c=1}^{13}\left(\frac{e_c}{\varepsilon_a+\varepsilon_r\max(|y_{n,c}|,\,|y_{n+1,c}|)}\right)^2}

:math:`\mathrm{err}\le1` ならステップを受理し、そうでなければ刻み幅を小さくして同じステップをやり直す。
次の刻み幅（やり直す場合はそのときの刻み幅）には、

.. math::
   :label: eq-step-factor

   h\leftarrow h\cdot f,\qquad f=\min\!\left(5,\ \max\!\left(0.2,\ 0.9\,\mathrm{err}^{-1/5}\right)\right)

を掛ける :cite:`hairer`。安全率は 0.9、倍率の範囲は 0.2〜5 である（:numref:`fig-step-factor`）。
やり直しで刻み幅が :math:`10^{-6}` s を下回ると、刻み幅が下限を下回ったとしてエラーで終了する（許容誤差を緩めるか ``rk4`` を使う）。
最初の刻み幅は ``sim.dt`` である。

.. _fig-step-factor:

.. figure:: /_generated/plots/step_factor.*
   :width: 75%

   規格化誤差と刻み幅の倍率。:math:`\mathrm{err}<0.9^5\approx0.59` なら広げ、:math:`\mathrm{err}>1` なら縮めてやり直す。
   倍率は :math:`\mathrm{err}<1.9\times10^{-4}` で上限の 5、:math:`\mathrm{err}>1.8\times10^{3}` で下限の 0.2 になる。

刻み幅は、制御された値と出力間隔 ``output_interval``\ （既定 0.05 s）のうち小さい方である。ランチャ上と燃焼中も 0.05 s が上限で、ランチャ上ではレールの終端にちょうど届く刻みにする。
このため、通常の飛翔では ``rtol``、``atol`` を小さくしても刻み幅が変わらず、結果がほとんど変わらないことがある。
推力曲線の折れ点、``sim.max_time``、開傘時刻をまたがない扱いは、``rk4`` の固定刻みにも同じように働く（:ref:`sec-step-events`）。

.. _sec-integrators-dop853:

Dormand–Prince 8(5,3) 法（``dop853``）
--------------------------------------

Prince と Dormand の 8 次の埋め込み公式 :cite:`princedormand` を用いる :cite:`hairer`。
刻み幅 :math:`h` の 1 ステップは 12 段で、8 次の解を伝播させる。

.. math::
   :label: eq-dop853

   \bm{k}_i=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\ (i=1,\dots,12),\qquad
   \bm{y}_{n+1}=\bm{y}_n+h\sum_{i=1}^{12}b_i\bm{k}_i

係数は Prince と Dormand の公式の値で、公表されている係数表と :math:`10^{-15}` で一致する。固定刻みの積分で観測される次数は 8 である（:ref:`page-vv-verification`）。
ランチャ離脱・頂点・着地の時刻は ``rk45`` と同じく、ステップ内の補間で求める。

誤差の推定には、5 次と 3 次の 2 つの埋め込み解の差 :math:`\bm{e}_5,\bm{e}_3` を使う。
それぞれを ``rk45`` と同じ規格化で二乗平均平方根にした :math:`\mathrm{err}_5,\ \mathrm{err}_3` から、

.. math::
   :label: eq-dop853-err

   \mathrm{err}=\frac{\mathrm{err}_5^{\,2}}{\sqrt{\mathrm{err}_5^{\,2}+0.01\,\mathrm{err}_3^{\,2}}}

を求める。5 次の推定を 3 次の推定で補正した形である。
刻み幅が小さいとき :math:`\mathrm{err}_5\propto h^6`、:math:`\mathrm{err}_3\propto h^4` なので :math:`\mathrm{err}\propto h^8` となり、刻み幅の制御に指数 :math:`1/8` を使う。
:math:`\mathrm{err}\le1` ならステップを受理し、次の刻み幅に

.. math::
   :label: eq-dop853-factor

   h\leftarrow h\cdot f,\qquad f=\min\!\left(6,\ \max\!\left(0.333,\ 0.9\,\mathrm{err}^{-1/8}\right)\right)

を掛ける。刻み幅の上限と下限の扱いは ``rk45`` と同じである。
1 ステップの右辺の評価は 12 回で、``rk45`` の 6 回の 2 倍である。

``dop853`` が有利になる場合
~~~~~~~~~~~~~~~~~~~~~~~~~~~

高次の方法は、解が滑らかで許容誤差が厳しいときに、少ない評価回数で同じ精度に達する。
空力係数が Mach 数に依らず迎角に線形で、風がなく、推力が 0 から始まって 0 で終わる滑らかな弾道飛翔（出力間隔 0.5 s）では、
``rk45`` と ``dop853`` の右辺の評価回数は、``rtol = atol = 1e-9`` で 7535 回と 4205 回、``1e-11`` で 17981 回と 6353 回であった（実測）。
許容誤差が厳しいほど差は開く。

一方、サンプルの設定ではこの利点は出ない（:numref:`tbl-integrators`）。
空力係数表は区分的に線形なので、Mach 数と迎角が格子点をまたぐところで右辺の微分が不連続になり、高次の方法でも刻み幅は大きくできない。
さらに、降下中は解が滑らかで、``rk45`` でも誤差の条件より先に出力間隔の上限（0.05 s）に達する。
ステップ数は ``rk45`` が約 1.39 万、``dop853`` が約 1.38 万とほとんど変わらず、1 ステップの評価回数が 12 回と 6 回なので、``dop853`` の評価回数は約 2 倍になる。
このサンプルでは、``rk45`` のほうが評価回数も時間も少ない。

積分法の比較
~~~~~~~~~~~~

サンプル（685 s のパラシュート降下を含む飛翔）で比較した結果を :numref:`tbl-integrators` に示す。時間は単一スレッドで計算した実測値である。
``rk4`` の :math:`\Delta t=0.5` ms を参照解とみなすと、:math:`\Delta t=2` ms の ``rk4`` と既定の許容誤差（``rtol=1e-7``, ``atol=1e-6``）の ``rk45`` は、
頂点高度で 0.002 m 以内、着地点で 0.010 m 以内で一致する。右辺の評価回数は ``rk45`` が約 1/16 で済む。
``dop853`` の評価回数は 164 986 回で、``rk45`` の約 2 倍である。

.. _tbl-integrators:

.. csv-table:: 積分法の比較（サンプル）
   :header-rows: 1
   :widths: 30 18 14 14 14 10

   積分法,評価回数,頂点 [m],着地点 東 [m],着地点 北 [m],時間 [s]
   "``rk4``, :math:`\Delta t=0.5` ms（参照）",5479648,4440.48,−655.57,−7707.69,0.69
   "``rk4``, :math:`\Delta t=2` ms（既定）",1369916,4440.48,−655.57,−7707.70,0.18
   "``rk4``, :math:`\Delta t=10` ms",273992,4440.51,−655.57,−7707.75,0.04
   "``rk45``, 既定の許容誤差",82972,4440.48,−655.57,−7707.70,0.018

``rk45`` の評価回数のほとんどは、出力間隔による上限（0.05 s）で決まり、:math:`685\ \mathrm{s}/0.05\ \mathrm{s}\times6\approx82\,000` 回がその大半を占める。
ランチャ離脱の時刻と速度は、どの積分法でも、レールの終端を越えたステップの中の補間で求めるので、刻み幅にほとんど依存しない。
``rk4`` は :math:`\Delta t=2` ms で 0.24715 s、45.661 m/s、:math:`\Delta t=10` ms で 0.24712 s、45.660 m/s、
参照解は 0.24715 s、45.661 m/s で、``rk45`` と ``dop853`` はともに 0.24715 s、45.661 m/s である。

.. _sec-integrators-lie:

Lie 群による姿勢の積分
----------------------

既定の ``attitude = "normalize"`` は、クォータニオンの 4 成分を通常の状態量として積分し、1 ステップごとに正規化する。
``"lie_group"`` では、姿勢を単位クォータニオンのなす群 :math:`S^3`\ （回転群 SO(3) の 2 重被覆）の上で直接進める。
Munthe-Kaas の Runge–Kutta–Munthe-Kaas（RKMK）法 :cite:`munthekaas98,munthekaas99` で、Lie 群上の積分の総説は :cite:`iserles,hairergni` にある。
更新は常に単位クォータニオンどうしの積なので、正規化をしなくても :math:`|\bm{q}|` は丸め誤差の範囲で 1 に保たれる。

指数写像と対数写像
~~~~~~~~~~~~~~~~~~

Lie 環 so(3) を :math:`\mathbb{R}^3` と同一視し、括弧積を外積とする。回転ベクトル :math:`\bm\theta`\ （rad）に対して

.. math::
   :label: eq-lie-exp

   \exp(\bm\theta)=\left(\cos\frac{|\bm\theta|}2,\ \sin\frac{|\bm\theta|}2\,\frac{\bm\theta}{|\bm\theta|}\right)

は、:math:`\bm\theta` まわりに角度 :math:`|\bm\theta|` だけ回す単位クォータニオンである。逆写像は、:math:`\bm{q}=(q_w,\bm{q}_v)` を :math:`q_w\ge0` となる符号にとって

.. math::
   :label: eq-lie-log

   \log(\bm{q})=\frac{2\,\mathrm{atan2}(|\bm{q}_v|,\,q_w)}{|\bm{q}_v|}\,\bm{q}_v

で、角度が :math:`[0,\pi]` の最短の回転を与える（:math:`\bm{q}` と :math:`-\bm{q}` は同じ回転）。

指数写像の微分
~~~~~~~~~~~~~~

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

:math:`\mathrm{dexp}^{-1}` は :math:`s=2\pi` で特異になるが、1 ステップの回転角はそれよりずっと小さい。

RKMK 法の 1 ステップ
~~~~~~~~~~~~~~~~~~~~

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
適応的な方法（``rk45``、``dop853``）の誤差の推定は、埋め込み解の重みをユークリッド部分と姿勢の Lie 環の増分の両方に掛けて求める。
姿勢の誤差は回転ベクトル（rad）で、2 つの解の姿勢の間の角度に等しい。許容誤差が小さいときは、姿勢の誤差が :math:`\varepsilon_a` rad 程度まで許される。
刻み幅の制御は、それぞれの方法のとおりである。

ランチャ上とパラシュート降下では姿勢の運動を解かず、:math:`\bm{q}` は一定である。ECEF（:ref:`sec-ecef`）でも、:math:`\bm\omega` は積分する座標系に対する機体座標での角速度なので、同じ式を使う。

トルクのない剛体（:math:`I=\mathrm{diag}(1,2,3)`）で確かめた結果を :numref:`fig-rigid-body-attitude` に示す。
RKMK 法はクォータニオンのノルムを丸め誤差の範囲で保ち、通常の状態量として積分した RK4 は 1 ステップごとにノルムの偏差を生じて、正規化がそれを取り除く。
姿勢の誤差は、刻み :math:`h=0.05` s で両者が同程度である。観測した次数と保存量の誤差の数値は :ref:`page-vv-verification` にある。

.. _fig-rigid-body-attitude:

.. figure:: /_generated/plots/rigid_body_attitude.*
   :width: 100%

   トルクのない剛体（:math:`I=\mathrm{diag}(1,2,3)`、刻み :math:`h=0.05` s）の姿勢の誤差（左）とクォータニオンのノルムの誤差（右）。
   参照解は RKMK 法（Dormand–Prince 5 次、:math:`h=1` ms）である。RK4 の「正規化前」は、毎ステップの正規化で取り除かれる偏差を示す。値がちょうど 0 の点は対数軸に描けないので省いた。

刻み幅・許容誤差・姿勢の更新法の影響
------------------------------------

刻み幅と許容誤差
~~~~~~~~~~~~~~~~

:numref:`tbl-integrator-accuracy` は、刻み幅と許容誤差を変えたときの結果の差である。

.. _tbl-integrator-accuracy:

.. csv-table:: 刻み幅と許容誤差の影響（サンプル、パラシュート降下）
   :header-rows: 1
   :widths: 36 16 16 16 16

   設定,頂点 [m],着地点 東 [m],着地点 北 [m],参照解との差（北）[m]
   ``rk4``、``dt`` = 0.5 ms（参照解）,4440.475,−655.57,−7707.69,―
   ``rk4``、``dt`` = 2 ms,4440.476,−655.57,−7707.70,0.01
   ``rk4``、``dt`` = 10 ms,4440.507,−655.57,−7707.75,0.06
   ``rk4``、``dt`` = 50 ms,4440.819,−655.62,−7708.29,0.60
   ``rk45``、``rtol = atol = 1e-3``,4440.498,−655.57,−7707.74,0.05
   ``rk45``、``rtol = atol = 1e-5``,4440.483,−655.57,−7707.71,0.02
   ``rk45``、``rtol = atol = 1e-9``,4440.475,−655.57,−7707.69,0.00
   ``dop853``、``rtol = atol = 1e-3``,4440.477,−655.57,−7707.70,0.01
   ``dop853``、``rtol = atol = 1e-9``,4440.475,−655.57,−7707.69,0.00

条件は ``examples/sample.toml`` の設定（射角 85°、北風 4 m/s、パラシュート降下）で、``sim.attitude = "normalize"`` である。
判断の目安は次のとおりである。

* ``dt`` は既定の 2 ms でよい。10 ms でも着地点は 0.1 m 以内に収まり、50 ms で 0.6 m 動く。0.05 s を超える値はエラーになる。
* ``rtol`` と ``atol`` は、サンプルでは ``1e-3`` まで緩めても着地点が 0.05 m 以内に収まる。
  既定の ``rtol = 1e-7``、``atol = 1e-6`` は十分に厳しい。
* 降下中は刻み幅が出力間隔 ``output_interval`` の上限で決まるので、許容誤差を変えても評価回数はほとんど変わらない（100 倍緩めて 82 972 回が 82 366 回）。
  速さを求めるときは、許容誤差より ``output_interval`` を変える。
* ``output_interval`` を 0.05 s から 5 s にしても、頂点と着地点の変化は頂点で 0.006 m、着地点で 0.01 m 以内である（``rk45``、``dop853``）。
  出力間隔は軌道の記録の粗さだけを決める。
* ``max_time`` を過ぎても着地しない飛翔は、着地なしの結果として出力され、終了ステータスは 3 になる（:ref:`sec-exit-status`）。既定は 3600 s である。

姿勢の更新法
~~~~~~~~~~~~

ロール角速度が大きいと、固定刻みでは 1 ステップの回転角が大きくなり、``normalize`` と ``lie_group`` の差が結果に出る。
:numref:`tbl-choose-attitude` は、サンプルにフィンのカント 1° を付けて速くロールさせた飛翔の結果である。

.. _tbl-choose-attitude:

.. csv-table:: 姿勢の更新法の影響（サンプル、カント 1°、最大ロール角速度 88 rad/s、パラシュート降下）
   :header-rows: 1
   :widths: 30 14 14 14 14 14

   積分法と刻み幅,``normalize`` 頂点 [m],``normalize`` 着地 北 [m],``lie_group`` 頂点 [m],``lie_group`` 着地 北 [m],1 ステップの回転角 [rad]
   ``rk4``、2 ms,4439.99,−7707.51,4440.35,−7708.16,0.18
   ``rk4``、5 ms,4436.15,−7700.60,4440.33,−7708.12,0.44
   ``rk4``、10 ms,4390.33,−7617.91,4440.38,−7708.26,0.88
   ``rk4``、20 ms,3763.51,−6426.45,4440.60,−7709.26,1.76
   ``rk45``\ （既定の許容誤差）,4440.35,−7708.15,4440.35,−7708.16,制御
   ``dop853``\ （既定の許容誤差）,4440.35,−7708.16,4440.35,−7708.16,制御

条件は ``examples/sample.toml`` に ``rocket.fin_cant_deg = 1.0`` を加えたものである。回転角は最大ロール角速度（時刻 2.95 s、Mach 1.54）に刻み幅を掛けた値である。
判断の目安は次のとおりである。

* ``rk45`` と ``dop853`` では、``normalize`` のままでよい。刻み幅が誤差で制御されるので、差は 0.01 m 以下である。
* ``rk4`` では、1 ステップの回転角（ロール角速度 × ``dt``）が 0.2 rad を超えるなら ``lie_group`` にする。
  ``normalize`` では 0.44 rad で頂点が 4 m、着地点が 8 m ずれ、0.88 rad では 90 m ずれる。``lie_group`` は 1.76 rad でも着地点が 1.1 m 以内である。
* ロール角速度が小さい機体（カントなし）では、どちらも同じ結果になる（サンプルで頂点の差は 1e-9 m の程度）。
* ``lie_group`` の計算時間は、``rk45`` で約 30 % 増え（18 ms に対して 23 ms）、``dop853`` ではほぼ同じである。どの組合せも頂点高度は参照解と 0.002 m 以内で一致する。

適用範囲と精度
--------------

全体の一覧は :ref:`sec-lim-motion` にある。数値積分に関わる境界は次のとおりである。

* ``rk45`` と ``dop853`` は、許容誤差を極端に小さくすると、刻み幅が下限（1 :math:`\mu\mathrm{s}`）を下回ってエラーになることがある。既定の許容誤差では起きない。
* 係数表は区分的に線形なので、Mach 数と迎角が格子点をまたぐ点で右辺の微分が不連続になり、高次の ``dop853`` でも刻み幅を大きくできない。
* ``dt`` は 10 :math:`\mu\mathrm{s}` から 0.05 s の範囲だけを受け付ける。

積分法の間の差は、サンプルで頂点 0.002 m、着地点 0.01 m 以内である（積分法どうしの相互比較）。
固定刻みの観測次数、Lie 群の方法の次数、ステップ内の事象と二段開傘の扱いは、解析解のある問題で確かめている（:ref:`page-vv-verification`）。

選び方
------

積分法と姿勢の更新法は独立に選べ、6 通りの組合せがすべて使える。決定の流れを :numref:`fig-integrator-choice` に、積分法ごとの使い分けを :numref:`tbl-choose-integrator` に示す。

.. _fig-integrator-choice:

.. figure:: /_generated/tikz/integrator_choice.*
   :width: 100%

   積分法（左）と姿勢の更新法（右）の選び方。太枠は既定（``rk4`` の ``dt = 2 ms`` と ``normalize``）である。見る点は、既定から変える理由が、
   「多数の飛翔を速く計算する（``rk45``）」「滑らかな係数で許容誤差の上限を優先する（``dop853``）」「大きなロール角速度（``lie_group``）」の三つに限られることである。

.. _tbl-choose-integrator:

.. list-table:: 積分法の選び方
   :header-rows: 1
   :widths: 24 38 38

   * - 積分法
     - 選ぶとき
     - 避けるとき
   * - ``rk4``\ （固定刻み ``dt``、既定 2 ms。範囲は 10 :math:`\mu\mathrm{s}`〜0.05 s）
     - 刻みを固定して他の計算と突き合わせる、参照解を作る。``dt`` = 2 ms で頂点 0.002 m、着地点 0.01 m 以内（参照解は 0.5 ms）
     - 多数の飛翔を計算する（``rk45`` の約 16 倍の評価）。大きなロール角速度を ``normalize`` で積分する
   * - ``rk45``\ （刻み幅制御 ``rtol``、``atol``）
     - 多数の飛翔（落下分散、最適化）を速く計算する。既定の許容誤差で頂点 0.002 m、着地点 0.010 m 以内
     - 許容誤差を極端に小さくする（刻み幅が下限 1 :math:`\mu\mathrm{s}` を下回るとエラー）
   * - ``dop853``\ （刻み幅制御 ``rtol``、``atol``）
     - 空力係数が滑らかな条件で許容誤差を厳しくする。``rtol = atol = 1e-9`` で ``rk45`` の 7535 回に対し 4205 回
     - サンプルのように係数表が区分的に線形で、出力間隔が刻みを抑える条件（``rk45`` より評価が多い）

関連する設定
------------

積分法は ``sim.integrator``\ （既定 ``"rk4"``）で選び、``sim.dt``\ （既定 0.002 s）は ``rk4`` だけに、``sim.rtol``、``sim.atol``\ （既定 1e-7、1e-6）は ``rk45`` と ``dop853`` だけに効く。
姿勢の更新法は ``sim.attitude``\ （既定 ``"normalize"``）で選ぶ。
``sim.output_interval``\ （既定 0.05 s）は軌道の記録間隔と適応法の刻み幅の上限、``sim.max_time``\ （既定 3600 s）は着地しない飛翔の打ち切り時刻である。
各キーの型と範囲は設定ファイルの章（:ref:`cfg-sim-integrator`）にある。
