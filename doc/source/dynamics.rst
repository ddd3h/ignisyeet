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

並進運動
--------

重心の運動は Newton の運動方程式に従う :cite:`goldstein`。

.. math::
   :label: eq-translation

   \dot{\bm{x}} = \bm{v},\qquad
   \dot{\bm{v}} = \frac{1}{m(t)}\,R(\bm{q})\left(\bm{F}_T + \bm{F}_A\right)_b + \bm{g}(z_U)

:math:`(\cdot)_b` は機体座標で表した量であることを示す。推力は機軸方向に働くとし、偏心やノズルの向きのずれは考えない。

.. math::

   \bm{F}_{T,b} = \left(T(t),\,0,\,0\right)^\top

推進剤の消費による質量の変化は :math:`m(t)` を通じて入る。:math:`\dot m\,\bm{v}` の項が現れないのは、推力 :math:`T` が実測値として
噴出ガスの運動量変化をすでに含んでいるからである :cite:`sutton`。

.. _sec-aero-forces:

空気力
~~~~~~

対気速度を機体座標で表したものを :math:`\bm{u}=R^\top(\bm{v}-\bm{w})=(u_x,u_y,u_z)` とし、

.. math::
   :label: eq-alpha

   V=|\bm{u}|,\qquad u_\perp=\sqrt{u_y^2+u_z^2},\qquad
   \alpha=\operatorname{atan2}(u_\perp,\,u_x),\qquad M=\frac{V}{a(h_0+z_U)}

とする。:math:`\alpha` はピッチとヨーを合わせた **全迎角**\ （:math:`0\le\alpha\le\pi`）である。
機体は軸対称なので、法線力は迎角の面内（機軸と対気速度を含む面）にあり、横向きの対気速度 :math:`(u_y,u_z)` と逆向きに働く。
係数表から :math:`(M,\alpha')`\ （:math:`\alpha'=\min(\alpha,\pi-\alpha)`）で係数を引き、:math:`q=\tfrac12\rho V^2` として

.. math::
   :label: eq-aero-force

   \bm{F}_{A,b} =
   -C_A\,q\,S\,\sgn(u_x)\begin{pmatrix}1\\0\\0\end{pmatrix}
   -C_N\,q\,S\,\frac{1}{u_\perp}\begin{pmatrix}0\\u_y\\u_z\end{pmatrix}

とする。燃焼中（:math:`T>0`）は :math:`C_A^{\mathrm{on}}`、燃焼後は :math:`C_A^{\mathrm{off}}` を使う。
:math:`\sgn(u_x)` により、機体が後ろ向きに飛ぶとき（:math:`u_x<0`）も軸力は常に対気速度に逆らう向きになる。

回転運動
--------

機体座標での角速度の変化は Euler の運動方程式に従う :cite:`goldstein`。慣性テンソルを :math:`I=\operatorname{diag}(I_{xx},I_{yy},I_{yy})` として

.. math::
   :label: eq-euler

   I\,\dot{\bm{\omega}} = \bm{M}_b - \bm{\omega}\times\left(I\bm{\omega}\right),\qquad
   \dot{\bm{q}} = \frac12\,\bm{q}\otimes(0,\bm{\omega})

である。第 2 項はジャイロ効果を表す。

空力モーメント
~~~~~~~~~~~~~~

法線力は圧力中心 :math:`x_{cp}` に働く :cite:`barrowman`。圧力中心の機体座標は :math:`(x_{cg}-x_{cp},0,0)` なので、復元モーメントは

.. math::
   :label: eq-moment-static

   \bm{M}_{N,b} = \begin{pmatrix}x_{cg}-x_{cp}\\0\\0\end{pmatrix}\times\bm{F}_{N,b}

である。:math:`x_{cp}>x_{cg}`\ （圧力中心が重心より後ろ）のとき、このモーメントは機首を対気速度の方向へ向ける。
これを **風見効果**\ （weathercocking）と呼ぶ。横風を受けると機体は風上を向くので、弾道落下の着地点は風上側にずれる。

これに\ :eq:`eq-damping` のピッチ減衰を加える。

.. math::
   :label: eq-moment-damp

   \bm{M}_{\mathrm{damp},b} = -\frac{qS}{V}\,\max\!\left(0,\ S_2-2x_{cg}S_1+x_{cg}^2S_0\right)\begin{pmatrix}0\\ \omega_y\\ \omega_z\end{pmatrix}

ロール方向のモーメントは 0 とする（フィンのカントやロール減衰は扱わない）。
したがって :math:`\bm{M}_b=\bm{M}_{N,b}+\bm{M}_{\mathrm{damp},b}` である。

飛翔の段階
----------

飛翔を :numref:`fig-phases` の段階に分け、段階ごとに異なる運動方程式を使う。

.. _fig-phases:

.. figure:: _generated/tikz/phases.*
   :width: 100%

   飛翔の段階と、段階が切り替わる条件。

ランチャ滑走
~~~~~~~~~~~~

ランチャ上では、機体はレールの方向 :math:`\bm{d}`\ （:eq:`eq-rail-dir`）にしか動けず、姿勢も変わらない。
レールに沿った速さを :math:`v_\parallel=\bm{v}\cdot\bm{d}` として

.. math::
   :label: eq-rail

   \dot{\bm{v}} = a_\parallel\,\bm{d},\qquad
   a_\parallel = \frac{T - C_A\,q\,S\,\sgn(v_\parallel)}{m} + \bm{g}\cdot\bm{d}

とし、:math:`v_\parallel\le0` かつ :math:`a_\parallel<0` のとき（推力が重力に負けて静止しているとき）は :math:`a_\parallel=0` とする。
レールに沿った移動距離が :math:`\bm{x}\cdot\bm{d}\ge L_{\mathrm{rail}}` になった時刻にランチャを離れ、自由飛行に移る。
このときの速さを **ランチャ離脱速度** として記録する。
燃焼が終わってもランチャを離れない場合は、推力不足としてエラーにする。

自由飛行
~~~~~~~~

:eq:`eq-translation` と :eq:`eq-euler` による 6 自由度の運動である。
弾道落下モードでは着地までこのまま計算する。

パラシュート降下
~~~~~~~~~~~~~~~~

頂点（:math:`v_z` の符号が正から負に変わった時刻 :math:`t_{\mathrm{apo}}`）から遅れ :math:`\Delta t` の後に開傘する。
開傘後は機体をパラシュートに吊られた質点とみなし、

.. math::
   :label: eq-parachute

   \dot{\bm{v}} = -\frac{\rho\,|\bm{v}_a|\,(C_DS)_p}{2m}\,\bm{v}_a + \bm{g},\qquad \bm{v}_a=\bm{v}-\bm{w}

とする :cite:`knacke`\ （:math:`(C_DS)_p` は設定 ``cd_s``）。終端降下速度は :math:`v_t=\sqrt{2mg/(\rho\,(C_DS)_p)}` で、
サンプル機体（燃焼後 8 kg、:math:`(C_DS)_p=3.5\ \mathrm{m^2}`）では海面付近で約 6.1 m/s になる。
開傘の衝撃や、開傘までの過渡的な挙動は扱わない。

着地
~~~~

ランチャ離脱後に :math:`z_U\le0` となったステップで計算を終える（ECEF モードでは測地的な高さ :math:`h-h_0\le0`）。
直前のステップとの間を線形補間し、
:math:`z_U=0` となる位置と時刻を着地点・着地時刻とする。着地点は\ :eq:`eq-enu-basis` と :eq:`eq-ecef-lla` で緯度・経度に変換する。

.. _sec-integrators:

数値積分
--------

状態方程式 :math:`\dot{\bm{y}}=\bm{f}(t,\bm{y})` を、設定 ``sim.integrator`` で選んだ方法で積分する。
どちらの方法でも、1 ステップの後でクォータニオンを正規化し、段階の切り替え（ランチャ離脱・頂点・開傘・着地）を判定する。
段階はステップの途中では変えない。

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

切り替えの時刻には最大 :math:`\Delta t` の誤差があるが、既定の :math:`\Delta t=2` ms では無視できる。
頂点の時刻は、:math:`v_z` の符号が変わったステップの終端とする。

推力曲線の折れ点や燃焼終了では、右辺 :math:`\bm{f}` が時間について滑らかでなくなる。そこでは局所的に 4 次の精度が出ないが、
:math:`\Delta t` を推力曲線の時間スケールより十分小さくとっているので、実用上の影響は小さい。
1 ステップで右辺を 4 回評価する。

Dormand–Prince 法（``rk45``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

刻み幅を誤差に応じて自動で調整する、埋め込み型の 5(4) 次 Runge–Kutta 法（Dormand–Prince 法 :cite:`dormand`）である :cite:`hairer`。
刻み幅 :math:`h` の 1 ステップは 7 段で、

.. math::
   :label: eq-dopri

   \bm{k}_i=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\ (i=2,\dots,6),\qquad
   \bm{y}_{n+1}=\bm{y}_n+h\sum_{j=1}^{6}b_j\bm{k}_j,\qquad
   \bm{k}_7=\bm{f}(t+h,\ \bm{y}_{n+1})

とする。係数は\ :eq:`eq-butcher` の Butcher 表 :cite:`dormand` のとおりで、5 次の解の重み :math:`b_j` は第 6 段の行（:math:`c_6=1`）と同じである。

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
段階が切り替わったステップの後は右辺が変わるので、:math:`\bm{k}_1` を評価し直す。
5 次の解 :math:`\bm{y}_{n+1}` のクォータニオンは、:math:`\bm{k}_7` を評価する前に正規化する。

誤差と刻み幅の制御
^^^^^^^^^^^^^^^^^^

誤差の推定は :math:`\bm{e}=h\sum_{j=1}^{7}E_j\bm{k}_j` で、13 個の状態量すべて（位置・速度・クォータニオン・角速度。単位や大きさの重み付けはしない）について
相対許容誤差 ``rtol``\ （:math:`\varepsilon_r`）と絶対許容誤差 ``atol``\ （:math:`\varepsilon_a`）で規格化し、二乗平均平方根をとる :cite:`hairer`。

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

   規格化誤差と刻み幅の倍率。誤差が許容値の 0.9 倍の 5 乗根より小さければ刻み幅を広げ、1 を超えると縮めてやり直す。

刻み幅の上限と調整
^^^^^^^^^^^^^^^^^^

各ステップの刻み幅は、制御された刻み幅と次の上限のうち小さい方である。

* 出力間隔 ``output_interval``\ （既定 0.05 s）。
* ランチャ上と燃焼中（:math:`t<t_b`）は 0.05 s。
* ランチャ上では、レールの終端に **ちょうど** 届くように合わせる。レールに沿った現在の速さ :math:`v_\parallel`、加速度 :math:`a_\parallel`、
  残りの距離 :math:`s_r=L_{\mathrm{rail}}-\bm{x}\cdot\bm{d}` から、加速度が一定として
  :math:`h\le\bigl(-v_\parallel+\sqrt{v_\parallel^2+2a_\parallel s_r}\bigr)/a_\parallel+10^{-6}` s とする（:math:`a_\parallel>0` かつ :math:`s_r>0` のとき）。
  これでランチャ離脱の時刻と速度が刻み幅ではなく :math:`10^{-6}` s 程度の精度で求まる。
* 燃焼中は、次の推力曲線の折れ点までの時間（折れ点をまたがない）。
* パラシュート降下で開傘前なら、開傘時刻までの時間（開傘の瞬間にステップの端を合わせる）。

上限で刻み幅を切り詰めたステップは、制御された刻み幅を小さくしない。
切り詰めたステップが受理されたときの次の刻み幅は :math:`\max(h_{\mathrm{ctrl}},\,h f)` で、
切り詰めのないとき（やり直しを含む）は :math:`h f` である。

頂点の時刻は、:math:`v_z` が正から 0 以下に変わったステップの中で線形補間した零点
:math:`t-h+h\,v_{z,\mathrm{prev}}/(v_{z,\mathrm{prev}}-v_z)` とする（刻み幅が大きいので、``rk4`` のようにステップの終端にはしない）。
開傘はこの時刻に ``delay`` を足した時刻に行う。着地の位置と時刻は、``rk4`` と同様に高さを線形補間して求める。
``trajectory.csv`` には、時刻が ``output_interval`` の倍数以上になった最初のステップの端点を書き出す（補間はしないので、間隔は完全には一定でない）。

``rk4`` と ``rk45`` の比較
^^^^^^^^^^^^^^^^^^^^^^^^^^

サンプルで比較した結果を :numref:`tbl-integrators` に示す。「降下」はパラシュート降下を含む約 685 s の飛翔、「弾道」は弾道落下の飛翔（約 62 s）の右辺の評価回数で、着地点と頂点は前者の値、時間は前者を単一スレッドで計算した実測値である。
``rk4`` の :math:`\Delta t=0.5` ms を参照解とみなすと、:math:`\Delta t=2` ms の ``rk4`` と既定の許容誤差（``rtol=1e-7``, ``atol=1e-6``）の ``rk45`` は、
頂点高度で 0.05 m 以内、着地点で 0.4 m 以内で一致する。右辺の評価回数は ``rk45`` が約 1/16 で済む。

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
     - 5479936
     - 495100
     - 4440.70
     - −654.81
     - −7708.13
     - 0.62
   * - ``rk4``, :math:`\Delta t=2` ms（既定）
     - 1369992
     - 123776
     - 4440.74
     - −654.82
     - −7708.46
     - 0.16
   * - ``rk4``, :math:`\Delta t=10` ms
     - 274012
     - 24756
     - 4440.95
     - −654.65
     - −7710.20
     - 0.03
   * - ``rk45``, 既定の許容誤差
     - 83205
     - 8414
     - 4440.66
     - −654.82
     - −7707.87
     - 0.015

``rk45`` の評価回数のほとんどは、出力間隔による上限（0.05 s）で決まる。降下中は解が滑らかなので誤差の条件より先に上限に達し、
:math:`685\ \mathrm{s}/0.05\ \mathrm{s}\times6\approx82\,000` 回がその大半を占める。
許容誤差を 100 倍に緩めても評価回数は 82 455 回とほとんど変わらないのはこのためである
（``output_interval`` を大きくすれば、さらに減らせる）。
一方、許容誤差を極端に小さくすると（例えば ``rtol = atol = 1e-9`` で弾道落下の着地直前）、刻み幅が下限を下回ってエラーになることがある。
ランチャ離脱は、``rk4`` では刻み :math:`\Delta t` の単位（0.248 s、45.84 m/s）、``rk45`` ではレール終端に合わせるので、
0.2472 s、45.66 m/s と細かく求まる（参照解の 0.2475 s、45.74 m/s に近い）。

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

   上昇中の Mach 数、迎角、静安定余裕、動圧。

.. list-table:: サンプルの主な結果
   :header-rows: 1
   :widths: 50 50

   * - 量
     - 値
   * - ランチャ離脱速度
     - 45.8 m/s（0.25 s）
   * - 最大速度 / 最大 Mach 数
     - 518.7 m/s / 1.54
   * - 最大動圧
     - 152 kPa
   * - 最小静安定余裕
     - 1.57 cal（ランチャ離脱直後）
   * - 頂点
     - 4441 m（26.1 s）
   * - 着地
     - 685 s、射点から 7.7 km

ランチャ離脱直後は速度が小さく、横風による迎角が大きい（約 6°）。迎角が大きいと胴体揚力（:eq:`eq-body-lift`）が圧力中心を前に移すので、
静安定余裕はこのとき最小になる。その後、推進剤の消費で重心が前に移り、超音速ではフィンの圧力中心が後退するので、安定余裕は 3.7 cal まで増える。
