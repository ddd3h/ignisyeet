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
どの方法でも、1 ステップの後で（``attitude = "normalize"`` ではクォータニオンを正規化したうえで）、段階の切り替え（ランチャ離脱・頂点・開傘・着地）を判定する。
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

   \bm{k}_i&=\bm{f}\!\left(t+c_ih,\ \bm{y}_n+h\sum_{j<i}a_{ij}\bm{k}_j\right)\quad (i=2,\dots,6),\\
   \bm{y}_{n+1}&=\bm{y}_n+h\sum_{j=1}^{6}b_j\bm{k}_j,\qquad
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
密な出力（dense output）の係数は使わない。頂点・着地の時刻は ``rk45`` と同じく、ステップ内の線形補間で求める。

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
（FSAL）。したがって 1 ステップあたりの右辺の評価は 13 回で、``rk45`` の 6 回の約 2 倍である。
段階が切り替わったステップの後は ``rk45`` と同じく :math:`\bm{k}_1` を評価し直す。
``attitude = "normalize"`` では、:math:`\bm{k}_{13}` はクォータニオンを正規化する前の :math:`\bm{y}_{n+1}` で評価する（正規化による差は許容誤差の程度である）。

``dop853`` が有利になる場合
^^^^^^^^^^^^^^^^^^^^^^^^^^^

高次の方法は、解が滑らかで許容誤差が厳しいときに、少ない評価回数で同じ精度に達する。
空力係数が Mach 数に依らず迎角に線形で、風がなく、推力が 0 から始まって 0 で終わる滑らかな弾道飛翔（出力間隔 0.5 s）では、
``rk45`` と ``dop853`` の右辺の評価回数は、``rtol = atol = 1e-9`` で 7532 回と 4202 回、``1e-11`` で 17978 回と 6350 回であった（実測）。
許容誤差が厳しいほど差は開く。

一方、サンプルの設定ではこの利点は出ない（:numref:`tbl-integrator-attitude`）。
空力係数表は区分的に線形なので、Mach 数と迎角が格子点をまたぐところで右辺の微分が不連続になり、高次の方法でも刻み幅は大きくできない。
さらに、刻み幅は出力間隔（0.05 s）で頭打ちになる。降下中は解が滑らかなので ``rk45`` でも誤差の条件より先にこの上限に達し、
ステップ数は ``rk45`` が約 1.39 万、``dop853`` が約 1.27 万とほとんど変わらない。1 ステップの評価回数が 13 回と 6 回なので、``dop853`` の評価回数は約 2 倍になる。
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
   参照解は RKMK 法（Dormand–Prince 5 次、:math:`h=1` ms）である。RK4 の「正規化前」は、毎ステップの正規化で取り除かれる偏差を示す。

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
     - 13（FSAL）
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
     - 4440.742
     - 1 369 992
     - 180
   * - ``rk45`` / ``normalize``
     - 4440.665
     - 83 205
     - 17
   * - ``rk45`` / ``lie_group``
     - 4440.669
     - 83 151
     - 24
   * - ``dop853`` / ``normalize``
     - 4440.677
     - 165 171
     - 46
   * - ``dop853`` / ``lie_group``
     - 4440.671
     - 165 219
     - 47
   * - ``rk4`` :math:`\Delta t=0.5` ms（参照）
     - 4440.697
     - ―
     - ―

どの組合せも、頂点高度は参照解と 0.05 m 以内で一致する。``lie_group`` は、``normalize`` に比べて ``rk45`` で時間が 7 ms（約 40 %）増える。
サンプルでは、姿勢の更新法による頂点高度の差は 0.01 m 以下で、計算時間は ``lie_group`` のほうが長い。
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
