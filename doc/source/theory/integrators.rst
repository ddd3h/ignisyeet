.. _page-theory-integrators:

数値積分と姿勢の更新
======================

運動方程式（:ref:`page-theory-dynamics`）を時間方向に解く方法と、姿勢（クォータニオン）の更新方法を述べる。
どの方法を選んでも結果の差は小さいが、ロール角速度の大きい機体と固定刻みの組合せでは差が大きくなる。この章はその境界を示す。

この章で決めること
------------------

* 積分法：:ref:`sim.integrator <cfg-sim-integrator>`\ （``"rk4"``、``"rk45"``、``"dop853"``）
* 固定刻みの刻み幅（``rk4`` のみ）：:ref:`sim.dt <cfg-sim-dt>`
* 刻み幅制御の許容誤差（``rk45``、``dop853`` のみ）：:ref:`sim.rtol <cfg-sim-rtol>`、:ref:`sim.atol <cfg-sim-atol>`
* 姿勢の更新法：:ref:`sim.attitude <cfg-sim-attitude>`\ （``"normalize"``、``"lie_group"``）
* 出力の間隔と打ち切り時間：:ref:`sim.output_interval <cfg-sim-output_interval>`、:ref:`sim.max_time <cfg-sim-max_time>`

選び方
------

:numref:`fig-integrator-choice` に決定の流れを示す。積分法と姿勢の更新法は独立に選べ、6 通りの組合せがすべて使える。

.. _fig-integrator-choice:

.. figure:: /_generated/tikz/integrator_choice.*
   :width: 100%

   積分法（左）と姿勢の更新法（右）の選び方。見る点は、既定の組合せ（``rk4`` の ``dt = 2 ms`` と ``normalize``）から変える理由が、
   「速さ（``rk45``）」「許容誤差の上限（``dop853``）」「大きなロール角速度（``lie_group``）」の三つに限られることである。

積分法
~~~~~~

.. _tbl-choose-integrator:

.. list-table:: 積分法の選び方
   :header-rows: 1
   :widths: 10 18 8 24 20 20

   * - 積分法
     - 刻み幅
     - 評価回数 / ステップ
     - 精度の目安（サンプル）
     - 選ぶとき
     - 避けるとき
   * - ``rk4``
     - 固定（``dt``。既定 2 ms、設定できる範囲は 10 µs〜0.05 s）
     - 4
     - ``dt`` = 2 ms で頂点 0.002 m、着地点 0.01 m 以内（参照解は ``dt`` = 0.5 ms）
     - 刻みを固定して他の計算と突き合わせる、参照解を作る
     - 多数の飛翔を計算する（``rk45`` の約 16 倍の評価）。大きなロール角速度を ``normalize`` で積分する
   * - ``rk45``
     - 制御（``rtol``、``atol``）
     - 6
     - 既定の許容誤差で頂点 0.002 m、着地点 0.010 m 以内
     - 通常の飛翔と落下分散
     - 許容誤差を極端に小さくする（刻み幅が下限 1 µs を下回るとエラー）
   * - ``dop853``
     - 制御（``rtol``、``atol``）
     - 12
     - 既定の許容誤差で頂点 0.002 m 以内。滑らかな条件では ``rtol = atol = 1e-9`` で ``rk45`` の 7535 回に対し 4205 回
     - 滑らかな条件で許容誤差を厳しくする
     - サンプルのように係数表が区分的に線形で、出力間隔が刻みを抑える条件（``rk45`` より評価が多い）

サンプルでの評価回数と時間は :numref:`tbl-integrators` に、``dop853`` が有利になる条件は :ref:`sec-integrators-dop853` にある。
滑らかな条件の評価回数は、空力係数が Mach 数に依らず迎角に線形で、風がなく、推力が 0 から 0 で終わる弾道飛翔（出力間隔 0.5 s）の実測である。

.. _fig-integrator-cost-accuracy:

.. figure:: /_generated/plots/integrator_cost_accuracy.*
   :width: 80%

   右辺の評価回数（横軸）と、参照解（``rk4``、:math:`\Delta t=0.5` ms）に対する着地点の差（縦軸）。
   サンプルのパラシュート降下で、``rk4`` は刻み幅、``rk45`` と ``dop853`` は許容誤差（``rtol = atol``、:math:`10^{-3}` から :math:`10^{-11}`）を変えた。
   同じ精度なら左にある方法が安い。差が :math:`10^{-4}` m 以下の点は下端に置いた。

刻み幅と許容誤差
~~~~~~~~~~~~~~~~

:numref:`tbl-integrator-accuracy` は、刻み幅と許容誤差を変えたときの結果の差である。

.. _tbl-integrator-accuracy:

.. list-table:: 刻み幅と許容誤差の影響（サンプル、パラシュート降下）
   :header-rows: 1
   :widths: 36 16 16 16 16

   * - 設定
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
     - 参照解との差（北）[m]
   * - ``rk4``、``dt`` = 0.5 ms（参照解）
     - 4440.475
     - −655.57
     - −7707.69
     - ―
   * - ``rk4``、``dt`` = 2 ms
     - 4440.476
     - −655.57
     - −7707.70
     - 0.01
   * - ``rk4``、``dt`` = 10 ms
     - 4440.507
     - −655.57
     - −7707.75
     - 0.06
   * - ``rk4``、``dt`` = 50 ms
     - 4440.819
     - −655.62
     - −7708.29
     - 0.60
   * - ``rk45``、``rtol = atol = 1e-3``
     - 4440.498
     - −655.57
     - −7707.74
     - 0.05
   * - ``rk45``、``rtol = atol = 1e-5``
     - 4440.483
     - −655.57
     - −7707.71
     - 0.02
   * - ``rk45``、``rtol = atol = 1e-9``
     - 4440.475
     - −655.57
     - −7707.69
     - 0.00
   * - ``dop853``、``rtol = atol = 1e-3``
     - 4440.477
     - −655.57
     - −7707.70
     - 0.01
   * - ``dop853``、``rtol = atol = 1e-9``
     - 4440.475
     - −655.57
     - −7707.69
     - 0.00

条件は ``examples/sample.toml`` の設定（射角 85°、北風 4 m/s、パラシュート降下）で、``sim.attitude = "normalize"`` である。
判断の目安は次のとおりである。

* ``dt`` は既定の 2 ms でよい。10 ms でも着地点は 0.1 m 以内に収まり、50 ms で 0.6 m 動く。0.05 s を超える値はエラーになる。
* ``rtol`` と ``atol`` は、サンプルでは ``1e-3`` まで緩めても着地点が 0.05 m 以内に収まる。
  既定の ``rtol = 1e-7``、``atol = 1e-6`` は十分に厳しい。
* 降下中は解が滑らかで、刻み幅が出力間隔 ``output_interval`` の上限で決まるので、許容誤差を変えても評価回数はほとんど変わらない（100 倍緩めて 82 972 回が 82 366 回）。
  速さを求めるときは、許容誤差より ``output_interval`` を変える。
* ``output_interval`` を 0.05 s から 5 s にしても、頂点と着地点の変化は頂点で 0.006 m、着地点で 0.01 m 以内である（``rk45``、``dop853``）。
  頂点・着地・開傘の時刻と状態は刻みの中の補間で求めるので、出力間隔は軌道の記録の粗さだけを決める。
* ``max_time`` を過ぎても着地しない飛翔は、着地なしの結果として出力され、終了ステータスは 3 になる（:ref:`sec-exit-status`）。既定は 3600 s である。

姿勢の更新法
~~~~~~~~~~~~

``normalize`` は、クォータニオンの 4 成分を通常の状態量として積分し、1 ステップごとに正規化する。
``lie_group`` は、姿勢を単位クォータニオンの群の上で直接進める（:ref:`sec-integrators-lie`）。
ロール角速度が大きいと、固定刻みでは 1 ステップの回転角が大きくなり、この差が結果に出る。
:numref:`tbl-choose-attitude` は、サンプルにフィンのカント 1° を付けて速くロールさせた飛翔の結果である。

.. _tbl-choose-attitude:

.. list-table:: 姿勢の更新法の影響（サンプル、カント 1°、最大ロール角速度 88 rad/s、パラシュート降下）
   :header-rows: 1
   :widths: 30 14 14 14 14 14

   * - 積分法と刻み幅
     - ``normalize`` 頂点 [m]
     - ``normalize`` 着地 北 [m]
     - ``lie_group`` 頂点 [m]
     - ``lie_group`` 着地 北 [m]
     - 1 ステップの回転角 [rad]
   * - ``rk4``、2 ms
     - 4439.99
     - −7707.51
     - 4440.35
     - −7708.16
     - 0.18
   * - ``rk4``、5 ms
     - 4436.15
     - −7700.60
     - 4440.33
     - −7708.12
     - 0.44
   * - ``rk4``、10 ms
     - 4390.33
     - −7617.91
     - 4440.38
     - −7708.26
     - 0.88
   * - ``rk4``、20 ms
     - 3763.51
     - −6426.45
     - 4440.60
     - −7709.26
     - 1.76
   * - ``rk45``\ （既定の許容誤差）
     - 4440.35
     - −7708.15
     - 4440.35
     - −7708.16
     - 制御
   * - ``dop853``\ （既定の許容誤差）
     - 4440.35
     - −7708.16
     - 4440.35
     - −7708.16
     - 制御

条件は ``examples/sample.toml`` に ``rocket.fin_cant_deg = 1.0`` を加えたものである。回転角は最大ロール角速度（時刻 2.95 s、Mach 1.54）に刻み幅を掛けた値である。
判断の目安は次のとおりである。

* ``rk45`` と ``dop853`` では、``normalize`` のままでよい。刻み幅が誤差で制御されるので、差は 0.01 m 以下である。
* ``rk4`` では、1 ステップの回転角（ロール角速度 × ``dt``）が 0.2 rad を超えるなら ``lie_group`` にする。
  ``normalize`` では 0.44 rad で頂点が 4 m、着地点が 8 m ずれ、0.88 rad では 90 m ずれる。``lie_group`` は 1.76 rad でも着地点が 1.1 m 以内である。
* ロール角速度が小さい機体（カントなし）では、どちらも同じ結果になる（サンプルで頂点の差は 1e-9 m の程度）。
* ``lie_group`` の計算時間は、``rk45`` で約 30 % 増える（:numref:`tbl-integrator-attitude`）。

モデル
------

.. _sec-integrators:

数値積分
~~~~~~~~

状態方程式 :math:`\dot{\bm{y}}=\bm{f}(t,\bm{y})` を、設定 ``sim.integrator`` で選んだ方法で積分する。
どの方法でも、1 ステップの後で（``attitude = "normalize"`` ではクォータニオンを正規化したうえで）、段階の切り替え（ランチャ離脱・頂点・開傘・着地）を判定する。
ステップの途中で起きる事象（ランチャ離脱・頂点・開傘・メインの開傘・着地）は、そのステップ内の補間で時刻と状態を求めて、次の項のとおりに扱う。
状態量に有限でない値が現れたときは、``error[IY-S081]: integration diverged at t = … s; reduce sim.dt or tighten rtol/atol`` というエラーで止める（終了コード 1。モンテカルロ法ではそのサンプルを失敗として数え、風の格子ではそのケースを失敗として記録する）。
値が有限でも、速さが 8 km/s を超えるか高さが 1000 km を超えた状態は、ロケットが到達しない数値的な発散とみなし、``(speed … m/s is not physical)`` または ``(altitude … m is not physical)`` を添えた同じ形のエラーで止める。

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
* 推力曲線の折れ点（燃焼終了を含む）。1 ステップの推力は、推力曲線の 1 つの区間の線形な値だけを使う。
* パラシュート降下で開傘前なら、開傘の時刻。

各事象の扱いは :ref:`sec-rail`、:ref:`sec-parachute`、:ref:`sec-dual-deploy`、:ref:`sec-landing` にある。

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

開傘、ランチャ離脱、頂点、着地の時刻は刻み幅に依らない（:ref:`sec-step-events`）。
刻みは推力曲線の折れ点と燃焼終了の時刻に合わせて切り詰めるので、最後のステップは :math:`\Delta t` より短くなりうる。
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

誤差と刻み幅の制御
~~~~~~~~~~~~~~~~~~

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

.. figure:: /_generated/plots/step_factor.*
   :width: 75%

   規格化誤差と刻み幅の倍率。:math:`\mathrm{err}<0.9^5\approx0.59` なら広げ、:math:`\mathrm{err}>1` なら縮めてやり直す。
   倍率は :math:`\mathrm{err}<1.9\times10^{-4}` で上限の 5、:math:`\mathrm{err}>1.8\times10^{3}` で下限の 0.2 になる。

刻み幅の上限と調整
~~~~~~~~~~~~~~~~~~

各ステップの刻み幅は、制御された刻み幅と次の上限のうち小さい方である。

* 出力間隔 ``output_interval``\ （既定 0.05 s）。
* ランチャ上と燃焼中（:math:`t<t_b`）は 0.05 s。

* ランチャ上では、レールの終端にちょうど届く刻みにする。

* 次の推力曲線の折れ点（燃焼終了を含む）までの時間（折れ点をまたがない）。
* ``sim.max_time`` までの時間。
* パラシュート降下で開傘前なら、開傘時刻までの時間（開傘の瞬間にステップの端を合わせる）。

最後の 3 つの上限は、``rk4`` の固定刻みにも同じように働く（:ref:`sec-step-events`）。

頂点の時刻
~~~~~~~~~~

頂点の時刻は、どの積分法でも、:math:`v_z` が正から 0 以下に変わったステップの中で、そのステップの三次エルミート補間（:eq:`eq-step-hermite`）の鉛直速度

.. math::
   :label: eq-apogee-vz

   v_z(s)=\frac{1}{h}\,\frac{\mathrm{d}\bm{x}}{\mathrm{d}s}\cdot\bm{e}_U\bigl(\bm{x}(s)\bigr)

が 0 になる点 :math:`s_a` から、:math:`t_{\mathrm{apo}}=t-h+h\,s_a` とする（:math:`\bm{e}_U` はその位置での上向きの単位ベクトル。平面地球では :math:`z_U` の向き）。
頂点の高さと水平位置は、同じ補間を :math:`s_a` で評価した値で、``apogee``、``apogee_east``、``apogee_north`` に記録する。
時刻と位置は同じ補間の上の点なので、補間した高さの極大になる。
開傘はこの時刻に ``delay`` を足した時刻に行う（:ref:`sec-parachute`）。着地の位置と時刻は、三次エルミート補間した高さの零点で求める（:ref:`sec-landing`）。
``trajectory.csv`` には、時刻が ``output_interval`` の倍数以上になった最初のステップの端点を書き出す（補間はしないので、間隔は完全には一定でない）。

``rk4`` と ``rk45`` の比較
~~~~~~~~~~~~~~~~~~~~~~~~~~

サンプルで比較した結果を :numref:`tbl-integrators` に示す。「降下」はパラシュート降下を含む約 685 s の飛翔、「弾道」は弾道落下の飛翔（約 62 s）の右辺の評価回数で、着地点と頂点は前者の値、時間は前者を単一スレッドで計算した実測値である。
``rk4`` の :math:`\Delta t=0.5` ms を参照解とみなすと、:math:`\Delta t=2` ms の ``rk4`` と既定の許容誤差（``rtol=1e-7``, ``atol=1e-6``）の ``rk45`` は、
頂点高度で 0.002 m 以内、着地点で 0.010 m 以内で一致する。右辺の評価回数は ``rk45`` が約 1/16 で済む。

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
     - 5479648
     - 494980
     - 4440.48
     - −655.57
     - −7707.69
     - 0.69
   * - ``rk4``, :math:`\Delta t=2` ms（既定）
     - 1369916
     - 123748
     - 4440.48
     - −655.57
     - −7707.70
     - 0.18
   * - ``rk4``, :math:`\Delta t=10` ms
     - 273992
     - 24756
     - 4440.51
     - −655.57
     - −7707.75
     - 0.04
   * - ``rk45``, 既定の許容誤差
     - 82972
     - 8301
     - 4440.48
     - −655.57
     - −7707.70
     - 0.018

``rk45`` の評価回数のほとんどは、出力間隔による上限（0.05 s）で決まる。降下中は解が滑らかなので誤差の条件より先に上限に達し、
:math:`685\ \mathrm{s}/0.05\ \mathrm{s}\times6\approx82\,000` 回がその大半を占める。
許容誤差を 100 倍に緩めても評価回数は 82 366 回とほとんど変わらないのはこのためである
（``output_interval`` を大きくすれば、さらに減らせる）。
一方、許容誤差を極端に小さくすると（例えば ``rtol = atol = 1e-9`` で弾道落下の着地直前）、刻み幅が下限を下回ってエラーになることがある。
ランチャ離脱の時刻と速度は、どの積分法でも、レールの終端を越えたステップの中の補間で求めるので、刻み幅にほとんど依存しない。
``rk4`` は :math:`\Delta t=2` ms で 0.24715 s、45.661 m/s、:math:`\Delta t=10` ms で 0.24712 s、45.660 m/s、
参照解は 0.24715 s、45.661 m/s で、``rk45`` と ``dop853`` はともに 0.24715 s、45.661 m/s である。

.. _sec-integrators-dop853:

Dormand–Prince 8(5,3) 法（``dop853``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Prince と Dormand の 8 次の埋め込み公式 :cite:`princedormand` を用いる :cite:`hairer`。
刻み幅 :math:`h` の 1 ステップは 12 段で、8 次の解を伝播させる。

.. math::
   :label: eq-dop853

   \bm{k}_i=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\ (i=1,\dots,12),\qquad
   \bm{y}_{n+1}=\bm{y}_n+h\sum_{i=1}^{12}b_i\bm{k}_i

係数 :math:`c_i,\,a_{ij},\,b_i` は Prince と Dormand の公式の値である（:math:`c_{12}=1`）。:math:`b_i` は :math:`i=1,6,\dots,12` だけが 0 でない。
係数は公表されている係数表と :math:`10^{-15}` で一致し、固定刻みの積分で観測される次数は 8 である（検証。:ref:`page-vv-verification`）。
ランチャ離脱・頂点・着地の時刻は ``rk45`` と同じく、ステップ内の補間で求める。

誤差の推定
~~~~~~~~~~

誤差の推定には、5 次と 3 次の 2 つの埋め込み解を使う。Prince と Dormand の公式の係数 :math:`\mathrm{er}_i` と、
8 次の重み :math:`b_i` から 3 次の解の重み :math:`\widehat b_i`\ （``bhh``。:math:`i=1,9,12` だけが 0 でない）を引いた :math:`b_i-\widehat b_i` を用いて、

.. math::
   :label: eq-dop853-e

   \bm{e}_5=h\sum_{i=1}^{12}\mathrm{er}_i\,\bm{k}_i,\qquad
   \bm{e}_3=h\sum_{i=1}^{12}\left(b_i-\widehat b_i\right)\bm{k}_i

とする。:math:`\bm{e}_5,\bm{e}_3` の各成分 :math:`c` を :math:`\mathrm{sc}_c=\varepsilon_a+\varepsilon_r\max(|y_{n,c}|,\,|y_{n+1,c}|)` で割り、
:math:`n=13` 個の成分の二乗平均平方根を :math:`\mathrm{err}_5,\ \mathrm{err}_3` とする。
:math:`h` を除いた二乗和 :math:`S_5=\sum_c\bigl(\sum_i\mathrm{er}_ik_{i,c}/\mathrm{sc}_c\bigr)^2`、
:math:`S_3=\sum_c\bigl(\sum_i(b_i-\widehat b_i)k_{i,c}/\mathrm{sc}_c\bigr)^2` から

.. math::
   :label: eq-dop853-err

   \mathrm{err}=\frac{|h|\,S_5}{\sqrt{n\,\bigl(S_5+0.01\,S_3\bigr)}}
   =\frac{\mathrm{err}_5^{\,2}}{\sqrt{\mathrm{err}_5^{\,2}+0.01\,\mathrm{err}_3^{\,2}}}

を求める。5 次の推定を 3 次の推定で補正した形である。
刻み幅が小さいとき :math:`\mathrm{err}_5\propto h^6`、:math:`\mathrm{err}_3\propto h^4` なので、:math:`\mathrm{err}\propto h^8` となり、
刻み幅の制御に指数 :math:`1/8` を使う根拠になる。

刻み幅の制御
~~~~~~~~~~~~

:math:`\mathrm{err}\le1` ならステップを受理し、次の刻み幅（やり直す場合はそのときの刻み幅）に

.. math::
   :label: eq-dop853-factor

   h\leftarrow h\cdot f,\qquad f=\min\!\left(6,\ \max\!\left(0.333,\ 0.9\,\mathrm{err}^{-1/8}\right)\right)

を掛ける。安全率は 0.9、倍率の範囲は 0.333〜6 で、:math:`\mathrm{err}=0` のときは 6 である。
直前のステップの誤差を使う安定化（:math:`\beta`）は使わない。刻み幅の上限と下限の扱いは ``rk45`` と同じである。
12 段の計算の後に、受理された点での右辺 :math:`\bm{k}_{13}=\bm{f}(t+h,\bm{y}_{n+1})` を 1 回評価し、次のステップの :math:`\bm{k}_1` として再利用する
（FSAL）。:math:`\bm{k}_1` は前のステップの値を使うので、1 ステップで新たに評価する右辺は :math:`\bm{k}_2,\dots,\bm{k}_{12}` の 11 回と :math:`\bm{k}_{13}` の 1 回の計 12 回で、``rk45`` の 6 回（:math:`\bm{k}_2,\dots,\bm{k}_7`）の 2 倍である。


``dop853`` が有利になる場合
~~~~~~~~~~~~~~~~~~~~~~~~~~~

高次の方法は、解が滑らかで許容誤差が厳しいときに、少ない評価回数で同じ精度に達する。
空力係数が Mach 数に依らず迎角に線形で、風がなく、推力が 0 から始まって 0 で終わる滑らかな弾道飛翔（出力間隔 0.5 s）では、
``rk45`` と ``dop853`` の右辺の評価回数は、``rtol = atol = 1e-9`` で 7535 回と 4205 回、``1e-11`` で 17981 回と 6353 回であった（実測）。
許容誤差が厳しいほど差は開く。

一方、サンプルの設定ではこの利点は出ない（:numref:`tbl-integrator-attitude`）。
空力係数表は区分的に線形なので、Mach 数と迎角が格子点をまたぐところで右辺の微分が不連続になり、高次の方法でも刻み幅は大きくできない。
さらに、刻み幅は出力間隔（0.05 s）で頭打ちになる。降下中は解が滑らかなので ``rk45`` でも誤差の条件より先にこの上限に達し、
ステップ数は ``rk45`` が約 1.39 万、``dop853`` が約 1.38 万とほとんど変わらない。1 ステップの評価回数が 12 回と 6 回なので、``dop853`` の評価回数は約 2 倍になる。
このサンプルでは、``rk45`` のほうが評価回数も時間も少ない。

.. _sec-integrators-lie:

Lie 群による姿勢の積分（``attitude = "lie_group"``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

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
:math:`|\bm\theta|` が小さいときは、:math:`\cos(|\bm\theta|/2)` と :math:`\sin(|\bm\theta|/2)/|\bm\theta|` を Taylor 展開で評価する。

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

これらの係数は :math:`s\to0` で 0 を 0 で割る形になるので、:math:`s<0.1` のときは級数

.. math::
   :label: eq-lie-series

   \frac{1-\cos s}{s^2}&=\frac12-\frac{s^2}{24}+\frac{s^4}{720}-\cdots,\\
   \frac{s-\sin s}{s^3}&=\frac16-\frac{s^2}{120}+\frac{s^4}{5040}-\cdots,\\
   \frac1{s^2}\left(1-\frac s2\cot\frac s2\right)&=\frac1{12}+\frac{s^2}{720}+\frac{s^4}{30240}+\cdots

で評価する。:math:`\mathrm{dexp}^{-1}` は :math:`s=2\pi` で特異になるが、1 ステップの回転角はそれよりずっと小さい。

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
自由な剛体（慣性主軸のモーメント :math:`I=\mathrm{diag}(1,2,3)`、:math:`\bm\omega_0=(0.3,\,1.0,\,0.4)` rad/s、トルクなし、:math:`t\in[0,20]` s）で
観測した次数は、RKMK 法の ``rk4`` の表で 4.10、Dormand–Prince 5 次の表で 4.92 である。

.. _fig-rigid-body-attitude:

.. figure:: /_generated/plots/rigid_body_attitude.*
   :width: 100%

   トルクのない剛体（:math:`I=\mathrm{diag}(1,2,3)`、刻み :math:`h=0.05` s）の姿勢の誤差（左）とクォータニオンのノルムの誤差（右）。
   参照解は RKMK 法（Dormand–Prince 5 次、:math:`h=1` ms）である。RK4 の「正規化前」は、毎ステップの正規化で取り除かれる偏差を示す。値がちょうど 0 の点は対数軸に描けないので省いた。

同じ問題の結果を :numref:`fig-rigid-body-attitude` に示す。刻み :math:`h=0.01` s の RKMK 法（``rk4`` の表）では、:math:`\bigl||\bm{q}|-1\bigr|` の最大値は :math:`5.8\times10^{-15}`、
エネルギーの相対誤差は :math:`7.1\times10^{-12}`、角運動量の大きさの相対誤差は :math:`4.6\times10^{-12}` である（実測）。
:math:`h=0.05` s では、姿勢の誤差は RKMK 法が :math:`1.57\times10^{-7}` rad、RK4 と正規化が :math:`2.59\times10^{-7}` rad で、同程度である。
この問題では、通常の状態量として積分した RK4 は 1 ステップごとに最大 :math:`1.9\times10^{-10}` のノルムの偏差を生じ、正規化がそれを取り除いている。

誤差の推定と刻み幅の制御
~~~~~~~~~~~~~~~~~~~~~~~~

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
~~~~~~~~~~~~

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
     - 4440.476
     - 1 369 916
     - 175
   * - ``rk45`` / ``normalize``
     - 4440.476
     - 82 972
     - 18
   * - ``rk45`` / ``lie_group``
     - 4440.475
     - 82 948
     - 23
   * - ``dop853`` / ``normalize``
     - 4440.475
     - 164 986
     - 44
   * - ``dop853`` / ``lie_group``
     - 4440.476
     - 164 830
     - 42
   * - ``rk4`` :math:`\Delta t=0.5` ms（参照）
     - 4440.475
     - ―
     - ―

どの組合せも、頂点高度は参照解と 0.002 m 以内で一致する。``lie_group`` は、``normalize`` に比べて ``rk45`` で時間が 5 ms（約 30 %）増える。
サンプルでは、姿勢の更新法による頂点高度の差は 0.001 m 以下で、計算時間は、``rk45`` では ``lie_group`` のほうが長く、``dop853`` ではほぼ同じである。
``lie_group`` が結果を大きく変えるのは、ロール角速度が大きいときの固定刻みである（上の「姿勢の更新法」）。

適用範囲と限界
--------------

全体の一覧は :ref:`sec-lim-motion` にある。

* ``rk45`` と ``dop853`` は、許容誤差を極端に小さくすると、刻み幅が下限（1 µs）を下回ってエラーになることがある。既定の許容誤差では起きない。
* 係数表は区分的に線形なので、Mach 数と迎角が格子点をまたぐ点で右辺の微分が不連続になり、高次の ``dop853`` でも刻み幅を大きくできない。
* 状態量に有限でない値が出た、速さが 8 km/s を超えた、高さが 1000 km を超えた飛翔は、数値的な発散としてエラーで止まる。落下分散では、そのケースを失敗として数える。
* ``dt`` は 10 µs から 0.05 s の範囲だけを受け付ける。

精度の根拠
----------

* 次数の確認：``rk4`` と ``dop853`` の固定刻みの観測次数、Lie 群の方法の次数（トルクなしの剛体で 4.10 と 4.92、:ref:`fig-rigid-body-attitude`）は、解析解のある問題で確かめている（検証。:ref:`page-vv-verification`）。
* ステップ内の事象は、真空中の解析解と比べ、刻み 0.05 s の ``rk4`` でも一致する（:ref:`sec-verify-events`）。
* 積分法の間の差は、上の表のとおり、サンプルで頂点 0.002 m、着地点 0.01 m 以内である（積分法どうしの相互比較）。

関連する設定キー
----------------

.. list-table::
   :header-rows: 1
   :widths: 30 16 54

   * - キー
     - 既定値
     - 効果の方向
   * - :ref:`sim.integrator <cfg-sim-integrator>`
     - ``"rk4"``
     - ``"rk45"`` で評価回数が約 1/16 になる。``"dop853"`` は厳しい許容誤差向き。
   * - :ref:`sim.dt <cfg-sim-dt>`
     - 0.002 s
     - ``rk4`` だけに効く。大きいほど速く、誤差が増える。
   * - :ref:`sim.rtol <cfg-sim-rtol>`、:ref:`sim.atol <cfg-sim-atol>`
     - 1e-7、1e-6
     - ``rk45`` と ``dop853`` だけに効く。小さいほど厳しい。
   * - :ref:`sim.attitude <cfg-sim-attitude>`
     - ``"normalize"``
     - ``"lie_group"`` は大きなロール角速度の固定刻みで誤差を抑える。
   * - :ref:`sim.output_interval <cfg-sim-output_interval>`
     - 0.05 s
     - 軌道の記録間隔と、適応法の刻み幅の上限。
   * - :ref:`sim.max_time <cfg-sim-max_time>`
     - 3600 s
     - 着地しない飛翔の打ち切り時刻。
