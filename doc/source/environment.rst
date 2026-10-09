.. _page-environment:

環境モデル
==========

飛翔の外部条件として、大気、重力、風の三つをモデル化する。大気は空気力の大きさと音速（Mach 数）を、重力は軌道そのものを、風は機体に対する空気の動きを決める。
どれも設定ファイルの ``[atmosphere]``、``[earth]``、``[wind]`` で選ぶ。

大気と重力は既定のままで多くの飛翔に足りる。気温が標準から大きく外れる日、高い頂点まで飛ぶ機体、他のコードとの比較では選択が結果に効く。
風は結果への影響が最も大きい。同じ地上風でも、高度分布の仮定を変えるだけで着地距離は数倍変わる（:numref:`tbl-wind-effect`）。

.. _page-theory-atmosphere-gravity:

大気
----

大気の温度・圧力・密度・音速・粘性係数は、既定では米国標準大気 1976 :cite:`us76` で与える（:numref:`fig-atmosphere`）。
飛翔計算での大気は **海抜高度** :math:`h_0+z_U` で評価する（ECEF モードでは楕円体高 :math:`h`）。

幾何高度 :math:`z`\ （海抜）を、重力の変化を考慮したジオポテンシャル高度

.. math::

   H = \frac{R_0\,z}{R_0+z},\qquad R_0=6\,356\,766\ \mathrm{m}

に直す :cite:`us76`。:math:`H` が層 :math:`k`\ （基準高度 :math:`H_k`、基準温度 :math:`T_k`、温度減率 :math:`L_k`、基準圧力 :math:`p_k`）にあるとき、

.. math::
   :label: eq-us76

   T = T_k + L_k\,(H-H_k),\qquad
   p = \begin{cases}
   p_k\left(\dfrac{T_k}{T}\right)^{g_0/(R L_k)} & (L_k\ne0)\\[8pt]
   p_k\exp\!\left(-\dfrac{g_0\,(H-H_k)}{R\,T_k}\right) & (L_k=0)
   \end{cases}

である。ここで :math:`g_0=9.80665\ \mathrm{m/s^2}`、:math:`R=287.05287\ \mathrm{J/(kg\,K)}` である。密度・音速・粘性係数は

.. math::
   :label: eq-atm-derived

   \rho = \frac{p}{RT},\qquad a=\sqrt{\gamma RT}\ \ (\gamma=1.4),\qquad
   \mu = \frac{1.458\times10^{-6}\,T^{3/2}}{T+110.4}\quad\text{（Sutherland の式）}

で求める。理想気体の関係式は標準的なもの :cite:`anderson` で、Sutherland の式 :cite:`sutherland` の係数は米国標準大気 1976 :cite:`us76` の値である。

.. list-table:: 米国標準大気 1976 の層（86 km まで）
   :header-rows: 1
   :widths: 22 22 26 30

   * - :math:`H_k` [km]
     - :math:`T_k` [K]
     - :math:`L_k` [K/km]
     - :math:`p_k` [Pa]
   * - 0
     - 288.15
     - −6.5
     - 101 325
   * - 11
     - 216.65
     - 0.0
     - 22 632.06
   * - 20
     - 216.65
     - +1.0
     - 5 474.889
   * - 32
     - 228.65
     - +2.8
     - 868.0187
   * - 47
     - 270.65
     - 0.0
     - 110.9063
   * - 51
     - 270.65
     - −2.8
     - 66.938 87
   * - 71
     - 214.65
     - −2.0
     - 3.956 420

86 km を超える高度では 86 km の値を使う。

.. _fig-atmosphere:

.. figure:: /_generated/plots/atmosphere.*
   :width: 100%

   米国標準大気 1976 の温度・密度・音速。温度は 11 km と 20 km、47 km で折れ、密度は高度とともにほぼ指数的に減る。

**温度差（``temperature_offset``）**

猛暑日や寒い日の打上げを想定して、標準大気の温度に高度によらない一定の温度差 :math:`\Delta T` [K] を加えることができる
（:numref:`fig-atmosphere-offset`）。層の境界のジオポテンシャル高度 :math:`H_k` と温度減率 :math:`L_k` は標準大気のままで、
基準温度だけが :math:`T_k'=T_k+\Delta T` に変わる。

温度を変えると、静水圧平衡を保つために基準圧力 :math:`p_k` も変わる。そこで海面の圧力を標準の 101 325 Pa に固定し、
:math:`\mathrm{d}p/\mathrm{d}H=-g_0\,p/(R\,T')` を層ごとに厳密に積分して :cite:`us76`、下の層から順に圧力を求める。
:math:`H` が層 :math:`k` にあるとき、層 :math:`j<k` は上端 :math:`H_{j+1}` まで、層 :math:`k` は :math:`H` まで積分して

.. math::
   :label: eq-us76-offset

   p(H) = p_0\prod_{j=0}^{k}\Pi_j,\qquad
   \Pi_j = \begin{cases}
   \left(\dfrac{T_j'+L_j\,(H_j^{+}-H_j)}{T_j'}\right)^{-g_0/(R L_j)} & (L_j\ne0)\\[8pt]
   \exp\!\left(-\dfrac{g_0\,(H_j^{+}-H_j)}{R\,T_j'}\right) & (L_j=0)
   \end{cases}

となる。ここで :math:`p_0=101\,325` Pa、:math:`H_j^{+}=\min(H,\,H_{j+1})`\ （最上層では :math:`H_j^{+}=H`）である。
温度は :math:`T=T_k+L_k(H-H_k)+\Delta T`、密度・音速・粘性係数は\ :eq:`eq-atm-derived` に :math:`T` を入れて求める。
:math:`\Delta T=0` のときは標準大気そのもの（:eq:`eq-us76`）である。\ :eq:`eq-us76-offset` を :math:`\Delta T=0` で評価しても、
標準大気の基準圧力 :math:`p_k`\ （表）を相対誤差 :math:`2\times10^{-5}` 以内で再現する。
:math:`\Delta T` は、モデルの範囲で最も低い標準温度 186.946 K に足しても絶対温度が正になるよう、:math:`-186.946` K より大きくなければならない。

海面の圧力が変わらないので、:math:`\Delta T=+15` K では海面の密度が標準より約 5 %\ （:math:`288.15/303.15` 倍）小さくなる。
空気が薄くなると抗力が減って頂点が高くなり、音速が大きくなるので同じ速度でも Mach 数が下がる。
``[atmosphere]`` の設定は摩擦抗力の Reynolds 数（射点高度）にも使われる。

.. _fig-atmosphere-offset:

.. figure:: /_generated/plots/atmosphere_offset.*
   :width: 100%

   温度差 :math:`\Delta T=\pm15` K を加えた大気。左が温度、右が標準大気に対する密度の差。高温側（:math:`+15` K）は密度が低く、差は海面付近で約 5 % である。

**一定大気（``model = "constant"``）**

密度 :math:`\rho`、音速 :math:`a`、粘性係数 :math:`\mu` を高度によらず一定とする（既定値は海面の標準値 1.225 kg/m\ :sup:`3`,
340.29 m/s, 1.789×10\ :sup:`-5` Pa s）。
温度と圧力は、これらと矛盾しないように :math:`T=a^2/(\gamma R)`、:math:`p=\rho R T` として与える。
密度、音速、粘性係数は、有限で正でなければならない。

.. _tbl-atmosphere-effect:

.. list-table:: 大気の設定を変えたときの結果の変化（サンプル機体、風 4 m/s（北）、パラシュート降下、``rk4``）
   :header-rows: 1
   :widths: 40 20 20 20

   * - 設定
     - 頂点 [m]
     - 着地距離 [m]
     - 基準との差（頂点）
   * - ``us1976``、:math:`\Delta T=0`\ （基準）
     - 4440.5
     - 7735.5
     - 
   * - :math:`\Delta T=+15` K
     - 4563.6
     - 7801.7
     - +123 m（+2.8 %）
   * - :math:`\Delta T=-15` K
     - 4315.1
     - 7667.1
     - −125 m（−2.8 %）
   * - ``constant``\ （海面の標準値 1.225 kg/m\ :sup:`3`、340.29 m/s）
     - 4000.2
     - 7761.4
     - −440 m（−9.9 %）

この表は ``examples/sample.toml`` に ``--set atmosphere.temperature_offset=15`` のように 1 項目ずつ上書きして得た。
温度差 15 K は海面の密度を約 5 % 変え、抗力が変わって頂点が約 3 % 動く。
一定大気は上空でも海面の密度を保つので、抗力が大きく出て頂点が約 1 割低くなる。実際の飛翔の予測には使えない。

.. _sec-gravity:

重力
----

重力のモデルは ``[earth]`` の ``gravity`` で選ぶ。
``constant`` と ``inverse_square`` は **有効重力**\ （地表で測る重力。自転による遠心力を含む）の大きさを与え、``j2`` は遠心力を含まない万有引力を与える。
ECEF 座標での扱いの違いは :ref:`sec-ecef-gravity` で述べる。

``constant``
   大きさ :math:`g_0=9.80665\ \mathrm{m/s^2}` 一定で、下向き（:math:`-\bm{e}_U`）に働く有効重力である。

``inverse_square``\ （既定）
   大きさを万有引力の法則に従って高度とともに減らす。

   .. math::
      :label: eq-gravity

      \bm{g}(z_U) = -g_0\left(\frac{R_\oplus}{R_\oplus+h_0+z_U}\right)^2\bm{e}_U,\qquad R_\oplus=6\,371\ \mathrm{km}

   :math:`h_0` は射点標高である。平面地球モードでは向きが ENU の :math:`-z_U` 方向、ECEF モードでは現在位置の測地的な下向き :math:`-\bm{e}_U` で、
   大きさは同じ法則（:math:`z_U` は射点標高からの高さ）で与える。これも有効重力（遠心力を含む）である。

``j2``\ （ECEF モードのみ）
   WGS84 の点質量に第 2 帯球調和係数 :math:`J_2` を加えた重力ポテンシャル :cite:`hofmann` から求める。
   ECEF 位置 :math:`\bm{r}=(X,Y,Z)^\top`、:math:`r=|\bm{r}|` として

   .. math::
      :label: eq-j2

      \bm{g}(\bm{r}) = -\frac{GM}{r^3}
      \begin{pmatrix}
      X\,\bigl[1+k\,(1-5\zeta^2)\bigr]\\
      Y\,\bigl[1+k\,(1-5\zeta^2)\bigr]\\
      Z\,\bigl[1+k\,(3-5\zeta^2)\bigr]
      \end{pmatrix},\qquad
      k=\frac32J_2\left(\frac{a}{r}\right)^2,\quad \zeta=\frac{Z}{r}

   定数は :math:`GM=3.986\,004\,418\times10^{14}\ \mathrm{m^3/s^2}`、:math:`J_2=1.082\,626\,68\times10^{-3}`、:math:`a=6\,378\,137\ \mathrm{m}` である :cite:`wgs84`。
   この式は万有引力だけで、遠心力は含まない（ECEF モードでは\ :eq:`eq-ecef-accel` の遠心加速度として別に加える）。
   有効重力は、この万有引力に遠心加速度 :math:`-\bm{\Omega}\times(\bm{\Omega}\times\bm{r})` を足したものに当たる（:eq:`eq-g-effective`）。
   鉛直方向以外にも小さな成分（赤道向きに約 :math:`3J_2g\sin\phi\cos\phi`）を持つ。
   平面地球モードでは地球の形が定義できないので、``gravity = "j2"`` はエラーになる。

:numref:`fig-gravity` のように、逆 2 乗則では高度 100 km で地表の約 97 %、300 km で約 91 % になる。
数 km 程度の飛翔では差は 0.1 % 程度で、風や空力係数の不確かさに比べて小さい。
計算コストはほとんど変わらないので、高い高度に到達するロケットにもそのまま使えるよう既定では逆 2 乗則を使う。
:math:`J_2` モデルの地表での大きさは、赤道で約 9.814 m/s\ :sup:`2`、極で約 9.832 m/s\ :sup:`2` である（:ref:`sec-verify-env`）。

.. _fig-gravity:

.. figure:: /_generated/plots/gravity.*
   :width: 80%

   高度による重力加速度の変化（逆 2 乗則）。高度 100 km で地表の約 97 %、300 km で約 91 % になる。見るところは、数 km の飛翔では曲線がほぼ水平で、重力の高度変化を考慮する必要が小さいことである。


.. _tbl-gravity-effect:

.. list-table:: 重力の設定を変えたときの結果の変化（サンプル機体、風 4 m/s（北）、パラシュート降下、``rk4``）
   :header-rows: 1
   :widths: 40 15 15 15 15

   * - 設定
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
     - 着地距離 [m]
   * - 平面地球、``inverse_square``\ （基準）
     - 4440.5
     - −655.6
     - −7707.7
     - 7735.5
   * - 平面地球、``constant``
     - 4438.8
     - −655.3
     - −7701.5
     - 7729.4
   * - ECEF、``constant``
     - 4437.9
     - −662.2
     - −7696.8
     - 7725.2
   * - ECEF、``inverse_square``
     - 4439.6
     - −662.5
     - −7702.9
     - 7731.4
   * - ECEF、``j2``
     - 4441.4
     - −662.7
     - −7710.3
     - 7738.7

この表も ``examples/sample.toml`` を ``--set earth.model=ecef --set earth.gravity=j2`` などで上書きして得た。
ECEF との組み合わせの制約と意味は :ref:`sec-ecef-gravity` にある。


.. _page-theory-wind:

風
--

.. _sec-wind:

風の表し方
~~~~~~~~~~

風モデルは ``[wind]`` の ``model`` で選ぶ。いずれも風は水平方向だけで、鉛直成分は 0 である。
:math:`h` は **地上高度**\ （射点標高からの高さ :math:`z_U`、:math:`h<0` では 0 として扱う）である。
基準風速 ``wind.speed`` は有限で 0 以上、風向 ``wind.direction_deg`` は有限でなければならない。

風向の定義
~~~~~~~~~~

風向 :math:`\psi_w` は気象の慣例 :cite:`wmo8` に従い、風が **吹いてくる** 方位を北から時計回りに測る（:numref:`fig-wind-dir`）。
例えば :math:`\psi_w=270^\circ` は西風（西から東へ吹く風）である。風速ベクトルは

.. math::
   :label: eq-wind-vec

   \bm{w}(h) = -w(h)\begin{pmatrix}\sin\psi_w\\ \cos\psi_w\\ 0\end{pmatrix}

で、鉛直成分は考えない。

.. _fig-wind-dir:

.. figure:: /_generated/tikz/wind_dir.*
   :width: 80%

   風向 :math:`\psi_w` と射方位 :math:`\psi` の定義。どちらも北から時計回りに測る。風向は風が吹いてくる側、射方位は機体が向かう側を指す。


``wind.speed`` は **基準高度 ``wind.ref_height`` での風速** である（既定の高度は 2 m）。
風速計を 10 m に立てて測ったなら、``ref_height = 10`` と測定値を ``speed`` に与える。
同じ測定値でも ``ref_height`` が違えば、上空の風速は変わる。
``power`` と ``log`` は基準高度の風を上空へ延ばす。10 m で測った風速を ``ref_height = 2`` のまま ``speed`` に入れると、上空の風を約 3 割過大に見積もる
（:math:`n=6` のとき :math:`(10/2)^{1/6}\approx1.31`）。

高度分布
~~~~~~~~

風速の高度分布は次の四つから選ぶ。

``power``\ （べき法則、既定）
   地表付近（大気境界層）でよく使われる分布で :cite:`counihan,stull`、

   .. math::
      :label: eq-wind

      w(h) = w_{\mathrm{ref}}\left(\frac{h}{h_{\mathrm{ref}}}\right)^{1/n}

   である。:math:`h_{\mathrm{ref}}` は基準高度 ``ref_height``\ （既定 2 m）、:math:`w_{\mathrm{ref}}` はその高度での風速 ``speed``、
   :math:`n` は地表の状態で決まる定数 ``exponent`` である。:math:`n` が小さいほど、高さとともに風が強くなる（:numref:`fig-wind-profile`）。

``constant``\ （一定風）
   高度によらず :math:`w(h)=w_{\mathrm{ref}}` とする。

``log``\ （対数則）
   中立な大気境界層の対数則 :cite:`stull` で、粗度長 :math:`z_0`\ （``roughness_length``、既定 0.03 m）を使って

   .. math::
      :label: eq-wind-log

      w(h) = w_{\mathrm{ref}}\,\frac{\ln\bigl(\max(h,\,z_0)/z_0\bigr)}{\ln(h_{\mathrm{ref}}/z_0)}

   とする。式の上では :math:`h\le z_0` で 0 になる（実際の地面付近の扱いは :ref:`sec-wind-blend`）。
   :math:`0<z_0<h_{\mathrm{ref}}/2` でなければならない（:numref:`fig-wind-models` 左）。満たさない設定は読み込みの段階でエラーになる。

``profile``\ （高度別の表）
   次項の表から、高度ごとの風を与える。

.. _fig-wind-profile:

.. figure:: /_generated/plots/wind_profile.*
   :width: 80%

   べき法則による風速の高度分布（:math:`w_{\mathrm{ref}}=4` m/s）。指数 :math:`n` が小さいほど上空の風が強く、地面付近で急に立ち上がる。

べき法則や対数則は本来、高度数百 m 程度までの大気境界層の近似である :cite:`stull`。
これを数 km まで延長すると、:numref:`fig-wind-profile` や :numref:`fig-wind-models` のように上空の風速を過大に見積もりやすい。
到達高度が高い機体の落下分散（特にパラシュート降下）は、この仮定に強く依存する。

地表の状態の目安を :numref:`tbl-wind-terrain` に示す :cite:`counihan,stull`。

.. _tbl-wind-terrain:

.. list-table:: 地表の状態と指数・粗度長の目安
   :header-rows: 1
   :widths: 40 30 30

   * - 地表の状態
     - ``exponent`` :math:`n`
     - ``roughness_length`` :math:`z_0` [m]
   * - 海上、開けた平地
     - 約 7
     - —
   * - 開けた草地
     - —
     - 0.01〜0.05
   * - 郊外
     - 約 4〜5
     - —
   * - 樹木や建物のある地表
     - —
     - 0.3〜1
   * - 都市部
     - 約 3
     - —

— は文献に値がないことを示す。


.. _sec-wind-blend:

地面付近のなめらかな接続
~~~~~~~~~~~~~~~~~~~~~~~~

``power`` と ``log`` の風速は、地面（:math:`h=0`）で 0 になる。``power`` の :math:`h^{1/n}` は :math:`h=0` で傾きが無限大になり、
``log`` は :math:`h=z_0` で 0 に折れる。このような点があると、刻み幅を制御する積分法（``rk45``、``dop853``）は許容誤差が小さいときに着地の直前で刻み幅を極端に小さくし、
刻み幅が下限を下回ってエラーになりうる。そこで、地面から高さ :math:`h_b`\ （``ground_blend_height``、既定 1 m）までは、風速を 3 次の Hermite 多項式 :math:`p(h)` に置き換える。
:math:`t=h/h_b` として、

.. math::
   :label: eq-wind-blend

   p(h)=w(h_b)\left(-t^3+t^2+t\right)+h_b\,w'(h_b)\left(t^3-t^2\right),\qquad 0\le h\le h_b

である。これは次の 4 つの条件を満たす 3 次式である。

.. math::

   p(0)=0,\qquad p'(0)=\frac{w(h_b)}{h_b},\qquad p(h_b)=w(h_b),\qquad p'(h_b)=w'(h_b)

高さ :math:`h_b` では風速も傾きもモデルの値と連続につながり、地面では風速が 0 で傾きが有限（:math:`w(h_b)/h_b`）になる。

端の傾きを :math:`w(h_b)/h_b` を単位として表し、地面で :math:`\alpha=1`、:math:`h_b` で :math:`\beta=h_b\,w'(h_b)/w(h_b)` とする。
3 次の Hermite 曲線は、:math:`\alpha^2+\beta^2\le9` のとき非負で単調に増える :cite:`fritsch`。
``power`` では :math:`\beta=1/n`、``log`` では :math:`\beta=1/\ln(h_b/z_0)` で、この条件は ``power`` で :math:`n\ge0.36`、``log`` で :math:`h_b\ge1.43\,z_0` なら必ず成り立ち、風速は :eq:`eq-wind-blend` のままである。
成り立たないとき（``log`` で :math:`h_b` が :math:`z_0` の近くにあるとき）は、:math:`\tau=3/\sqrt{1+\beta^2}` として両端の傾きを :math:`\tau\alpha`、:math:`\tau\beta` に縮める（Fritsch–Carlson の補正）。

.. math::
   :label: eq-wind-blend-scaled

   p(h)=w(h_b)\Bigl[\tau\left(t^3-2t^2+t\right)+\left(-2t^3+3t^2\right)+\tau\beta\left(t^3-t^2\right)\Bigr]

こうすると、風速は 0 以上で、地面から :math:`h_b` まで単調に増える。:math:`h_b` で風速はモデルの値と連続だが、傾きはモデルの傾きより小さくなり、折れ目を生じる。
:math:`h\ge h_b` の風速は、:eq:`eq-wind` や :eq:`eq-wind-log` のままである。
``ground_blend_height`` は正で ``ref_height`` より小さい値でなければならない。``log`` では :math:`z_0` より大きい必要があり、
設定値が :math:`z_0` 以下なら :math:`\max(h_b,\,2z_0)` に読み替える。``constant`` と ``profile`` には適用しない。
サンプル（``power``、:math:`h_b=1` m）では、この接続による着地点の変化は約 1 cm である。

.. _sec-wind-profile:

高度別の風の表（``profile``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ラジオゾンデや数値気象モデル（GFS, ECMWF など）から取り出した風は、CSV ファイルにして ``wind.profile`` で指定する（作り方は :ref:`page-howto-environment`）。
表の列は、地上高度 [m]（射点標高からの高さ。0 以上で、**厳密に増加**）、風速 [m/s]（0 以上）、
風が吹いてくる方位 [deg]（北から時計回り。0 以上 360 以下）である。

中間の高度では、風速と風向を別々に補間するのではなく、風速ベクトルの成分を高度について **線形補間** する。

.. math::
   :label: eq-wind-profile

   \bm{w}_i=-w_i\begin{pmatrix}\sin\psi_i\\ \cos\psi_i\end{pmatrix},\qquad
   \bm{w}(h)=\bm{w}_i+\frac{h-h_i}{h_{i+1}-h_i}\,(\bm{w}_{i+1}-\bm{w}_i)\qquad(h_i\le h<h_{i+1})

風向が大きく違う 2 点の間では、補間した風速が両端より小さくなる（例えば 350° と 10° の同じ風速 :math:`w` の中間は、方位 0° で風速 :math:`w\cos10^\circ` である）。
表の最下端より低いところでは最下端の値、最上端より高いところでは最上端の値を **一定** に保つ（外挿しない）。
:numref:`fig-wind-models` 右は、表の例の成分を補間したものである。

``profile`` では ``speed`` と ``direction_deg`` は使わない（設定ファイルに書いてあると、使わないことを警告する）。代わりに、表全体の代表風を基準風として扱う。
代表風速 :math:`W` は、表の最下行から最上行までの高度範囲での平均風速

.. math::
   :label: eq-wind-profile-mean

   W=\frac{1}{h_N-h_1}\int_{h_1}^{h_N}\bigl|\bm{w}(h)\bigr|\,\mathrm{d}h

であり（1 行だけの表ではその行の風速）、代表風向 :math:`\Psi` は、同じ範囲で平均した風速ベクトル
:math:`\overline{\bm{w}}=\frac{1}{h_N-h_1}\int\bm{w}\,\mathrm{d}h` が吹いてくる方位である。
平均ベクトルがほぼ 0 のとき（互いに逆向きの風が打ち消し合うとき）は、基準高度 ``ref_height`` での風向を使う（そこも無風なら 0°）。
:math:`W` はベクトルの大きさの平均なので、風向が高度で変わる表では平均ベクトルの大きさ :math:`|\overline{\bm{w}}|` より大きい。
:math:`W` は ``ignisyeet sim`` の ``summary.json`` の ``wind_speed`` に出る。

:math:`W` は表の全範囲での平均なので、頂点よりずっと高い行まで表が続くと、飛翔に関係しない上空の風が :math:`W` に入る。
表を飛翔の高度範囲に切ると、:math:`W` と、それを基にした風速格子やモンテカルロ法の標準偏差の解釈が容易になる。
切るときは、公称の頂点のすぐ上ではなく、風の格子とモンテカルロ解析で予想される最も高い頂点（たとえば公称の頂点 + 3σ。σ はモンテカルロ解析の頂点高度の標準偏差 ``apogee_std``）より上まで残す。
表より上の風は最上端の値で一定になるので、頂点が最上端を超える飛翔は、その上の風を表していない。頂点が最上端を超えたケースとサンプルは、数と最も高い頂点が警告に出る（:ref:`IY-O522`）。

.. _fig-wind-models:

.. figure:: /_generated/plots/wind_models.*
   :width: 100%

   風モデル。左は一定風・べき法則・対数則（:math:`w_{\mathrm{ref}}=4` m/s, :math:`h_{\mathrm{ref}}=2` m）で、高さとともに風速が増える割合がモデルで違う。
   右は高度別の表の例（点が表の値、線が成分の線形補間）で、風向の変化を含む。

モデルの違いが結果に効く大きさ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

:math:`w(h)/w_{\mathrm{ref}}` は、基準高度（2 m）の風速に対する高度 :math:`h` の風速の比で、式 :eq:`eq-wind` と :eq:`eq-wind-log` による。
同じ地上風でも、モデルと指数で上空の風速が大きく違う。
1000 m では、``power`` の指数 4、6、7 で 4.73、2.82、2.43 倍、``log``\ （:math:`z_0=0.03` m）で 2.48 倍になり、``constant`` は高さによらず 1 倍である。
4000 m では指数 4 が 6.69 倍、指数 7 が 2.96 倍、``log`` が 2.81 倍である。

:numref:`tbl-wind-effect` は、その違いが着地点にどう出るかを示す。

.. _tbl-wind-effect:

.. list-table:: 風モデルを変えたときの結果の変化（サンプル機体、北から 4 m/s（高度 2 m）、パラシュート降下、``rk4``）
   :header-rows: 1
   :widths: 40 15 15 15 15

   * - 設定
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
     - 着地距離 [m]
   * - 無風
     - 4477.8
     - −663.9
     - 0.0
     - 663.9
   * - ``constant``
     - 4466.4
     - −661.5
     - −2372.3
     - 2462.8
   * - ``log``、:math:`z_0=0.03` m
     - 4441.6
     - −656.1
     - −6392.4
     - 6426.0
   * - ``power``、:math:`n=7`
     - 4446.7
     - −657.0
     - −6523.8
     - 6556.8
   * - ``power``、:math:`n=6`\ （既定）
     - 4440.5
     - −655.6
     - −7707.7
     - 7735.5
   * - ``power``、:math:`n=4`
     - 4405.3
     - −647.6
     - −13750.7
     - 13765.9

この表は ``examples/sample.toml`` に ``--set wind.model=log`` や ``--set wind.exponent=4`` のように 1 項目ずつ上書きして得た。
東向きのずれは風によらず約 −660 m で、残りは機体の運動（風見効果）による。
地上風が同じでも、``power`` の指数 4 と 7 で着地距離が 2 倍以上違う。
パラシュート降下の機体は上空の風で流されるので、頂点が高いほど風モデルの選択が効く。

``profile`` では、風向が高度で変わる表を使うと、着地の方向そのものが変わる。
``examples/wind_profile_example.csv``\ （高度 0〜6000 m、風向は地上の 10° から上空の 265° へ回る例。実測ではない）を同じ機体で飛ばすと、着地点は東 −272 m、北 +4384 m（着地距離 4393 m）で、
一方向の風 ``power`` の結果（南へ 7.7 km）と向きも距離も異なる。
風向の変化は、風速格子（``wind_grid``）の 1 方向だけの風では表現できない。


.. _sec-wind-scaling:

落下分散のための基準風の変更
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

落下分散（:ref:`page-theory-dispersion`）では、基準風速と基準風向だけを変えた風を何度も作る。
``constant``, ``power``, ``log`` では ``speed`` と ``direction_deg`` を差し替えるだけである。したがって風速格子の値は基準高度 ``ref_height`` での風速である。
``profile`` では、表の形（高度ごとの風速比と風向のねじれ）を保つために、代表風速を :math:`W\to W'`、
代表風向を :math:`\Psi\to\Psi'` に変えるとき、

.. math::
   :label: eq-wind-scale

   w_i' = k\,w_i,\quad k=\frac{W'}{W},\qquad
   \psi_i' = (\psi_i+\Psi'-\Psi)\bmod 360^\circ

と、表の **すべての** 風速を同じ比 :math:`k` で拡大し、すべての風向を同じ角度だけ回転する。
結果の表の代表風速は :math:`W'`、代表風向は :math:`\Psi'` になる。
風速格子（``wind_speeds``、``directions``）の値は、``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` に対する値である。
地上 2 m の風ではないので、地上風の測定値を風速格子にそのまま使ってはならない（選び方は :ref:`sec-dispersion-safety`）。

表の平均風を別の値にして飛ばすには、``wind.profile_mean_speed`` に平均風速 :math:`W'` [m/s] を、``wind.profile_mean_direction_deg`` に平均風の風向 :math:`\Psi'` [deg] を与える。
表は :eq:`eq-wind-scale` で拡大・回転され、風速 :math:`W'`、風向 :math:`\Psi'` の風の格子のケースと同じ風になる。片方だけを指定すると、もう片方は表のまま（:math:`W'=W` または :math:`\Psi'=\Psi`）である。
``sim`` もこの風で飛び、``summary.json`` の ``wind_speed`` と ``wind_direction_deg`` は :math:`W'` と :math:`\Psi'`、``wind_scale`` は :math:`k=W'/W` になる。
風の格子とモンテカルロ法は、この拡大・回転した表を公称の風として、そこからさらに :eq:`eq-wind-scale` で風速と風向を変える（拡大率はいつも CSV の表に対する値である）。
落下分散の格子で最も遠くなった風（平均風速と風向）に、機体のばらつきを与えて計算するときに使う（:ref:`sec-dispersion-safety`）。

モンテカルロ法の風速の揺らぎ :math:`\delta`\ （``wind_speed``、単位 m/s）は、``profile`` では平均風速 :math:`W` に対する揺らぎであり、
:math:`k=\max(W+\delta,\,0)/W` で表を拡大する。:math:`k\ge0` なので、揺らぎで風向が反転することはない。

平均風速が 0.1 m/s 未満（ほぼ無風）の表は拡大・回転できない。風速格子、および風速の揺らぎが 0 でないモンテカルロ法ではエラーになる。
風速格子では ``model = "constant"`` または ``"power"`` で格子を作り、モンテカルロ法では風速と風向の標準偏差を 0 にするか、風のある表を使う。
単一の飛翔（``ignisyeet sim``）は拡大しないので、ほぼ無風の表でも実行できる。

拡大率 :math:`k` が 3 を超える、または 1/3 を下回るケースがあると、警告を出す。
表の風は上空ほど強いのが普通で、拡大は上空の風速にも同じ比で効くので、:math:`k` が大きいと上空の風が非現実的に強くなるからである。
``summary.json`` の ``wind_scale`` は、その飛翔で使った :math:`k`\ （CSV の表そのままなら 1）である。``profile`` 以外のモデルでは出力しない。
表の最下行が地上高度 0 m より上にあるときは、それより下の風は最下行の値に保たれる（警告を出す）。



落下分散では、基準風を拡大して多くの風で計算する。基準風の意味はモデルで異なる。

.. list-table:: 風の格子の風速 ``dispersion.wind_speeds`` の意味
   :header-rows: 1
   :widths: 24 76

   * - 風モデル
     - 格子の値の意味
   * - ``constant``、``power``、``log``
     - 基準高度 ``ref_height`` での風速。上空の風速は、モデルに従ってその値から決まる
   * - ``profile``
     - 表の平均風速 :math:`W`\ （:eq:`eq-wind-profile-mean`）に対する値。表の形（高度ごとの風速比と風向のねじれ）を保って拡大する。地上風の測定値をそのまま格子に使ってはならない

:math:`W=11.89` m/s の ``examples/wind_profile_example.csv`` を ``profile_mean_speed`` で拡大して飛ばすと（サンプル機体、``sim``）、着地の北向きの距離は拡大率にほぼ比例して増え、頂点はほとんど変わらない。
たとえば 3 m/s（:math:`k=0.25`）で北 1107.7 m、6 m/s で 2215.2 m、指定なし（11.89 m/s）で 4384.1 m、15 m/s（:math:`k=1.26`）で 5528.8 m になり、東向きのずれは −565.2 m から −173.4 m へ縮む。
風速格子の ``wind_speeds`` の値も、``profile`` ではこの平均風速 :math:`W'` として同じ倍率 :math:`k=W'/W` で表に掛かる。

``ground_blend_height``\ （``power`` と ``log`` の地面近くのなめらかな接続の高さ）は、既定の 1 m から 0.25〜1.5 m に変えても、サンプルの着地点は 0.03 m しか動かない。
値を変える理由は通常ない。変えるのは、設定値が ``ref_height`` 以上か ``log`` の粗度長以下になってエラーや読み替えが出るときだけである。

適用範囲と精度
--------------

大気は、米国標準大気 1976 :cite:`us76` の 86 km までの層で与える。温度差は高度によらず一定の値を加えるだけで、逆転層や湿度、気圧の日変化は入らない（海面気圧は 101 325 Pa に固定）。
摩擦抗力の Reynolds 数は射点高度の大気で固定しており、高度による変化を含まない（:ref:`sec-lim-aero`）。
大気の設定を変えると係数表も作り直されるが、外部の表（``method = "table"``）は更新されない（:ref:`sec-lim-env`）。
標準大気と温度差を加えた大気は、文献値と静水圧平衡の数値積分に一致することを確かめている（:ref:`sec-verify-env`）。

重力は点質量に :math:`J_2` 項を加えたところまでで、高次の項や重力異常は考えない :cite:`hofmann`。
``constant`` と ``inverse_square`` は緯度による変化を含まない（:ref:`sec-lim-earth`）。``j2`` は ECEF モードでのみ使える。
:math:`J_2` 重力は、地表の赤道と極の大きさを確かめている（検証）。

風は水平方向だけで、時間によらない定常な風である。突風や乱流は考えない。
``power`` と ``log`` は大気境界層（数百 m まで）の近似で、上空の風速を過大に見積もりやすい。頂点が地上 1000 m を超えると、これらのモデルが上空へ外挿されることを警告する（:ref:`IY-O514`）が、結果は修正しない。
``profile`` の表の範囲外は端の値で一定とし、時刻や水平位置による変化は表現できない。
ECEF モードの風は、機体の現在位置の局所 ENU で与える。射点の基底との差は射点からの距離を地球半径で割った角度（55 km で約 0.5°）で、風の不確かさに比べて無視できる。
風の格子とモンテカルロ法の拡大は、表の形を保つ近似であり、風の鉛直構造の変化は表現しない。詳しい一覧は :ref:`sec-lim-env` にある。

風モデルは、基準高度での一致、対数則の値、``profile`` の補間と端の値の保持、基準風の拡大・回転、地面付近の接続を解析解や定義と比べて確かめている（:ref:`sec-verify-env`）。

大気、重力、風のいずれも、実測と比べた妥当性確認はない。べき法則と対数則が実際の上空の風をどこまで表すかは、風の観測で別に確かめる必要がある。

選び方
------

風モデルは :numref:`fig-choose-wind-model` の流れ図で選ぶ。大気と重力は既定のままでよい場合が多く、変える状況を :numref:`tbl-choose-environment` に挙げる。

.. _fig-choose-wind-model:

.. figure:: /_generated/tikz/wind_model_choice.*
   :width: 80%

   風モデルの選び方。上から順に答えると、観測または予報があれば ``profile``、なければ頂点の高さと粗度長の有無で ``power`` か ``log`` に至る。橙の箱は上空へ外挿する選択で、結果は参考値にとどまる。

.. _tbl-choose-environment:

.. list-table:: 大気と重力を既定から変える状況
   :header-rows: 1
   :widths: 28 22 50

   * - 設定
     - 値
     - 選ぶとき・避けるとき
   * - 大気
     - ``us1976``\ （既定）
     - 気温の実測がない、または標準的な気温の日。
   * - 大気
     - ``temperature_offset``
     - 打上げ日の気温が標準（15 °C）と大きく違う、または季節差を見るとき。気圧が標準から大きく外れる日には使えない。
   * - 大気
     - ``constant``
     - 風洞条件や他のコードとの比較で大気の影響を切り離したいとき。実際の飛翔の予測には選ばない。
   * - 重力
     - ``inverse_square``\ （既定）
     - 平面地球・ECEF のどちらでも使える。迷ったらこれ。
   * - 重力
     - ``constant``
     - 頂点が低い飛翔の検証や、解析解との比較。高い頂点には避ける。
   * - 重力
     - ``j2``
     - ECEF モードで、緯度による重力差まで含めたいとき。

関連する設定
------------

大気は ``atmosphere.model`` と ``temperature_offset`` で選び、``constant`` では ``density``、``sound_speed``、``viscosity`` を与える。
重力は ``earth.gravity`` で選ぶ。
風は ``wind.model`` で高度分布を選び、``speed``、``direction_deg``、``ref_height`` で基準風を与える。
``power`` は ``exponent``、``log`` は ``roughness_length``、``power`` と ``log`` の地面付近は ``ground_blend_height`` で調整し、``profile`` は ``profile`` に表を指定して ``profile_mean_speed``、``profile_mean_direction_deg`` で平均風を目標に合わせる。
各キーの型、既定値、値の範囲は :ref:`cfg-wind` と :ref:`cfg-atmosphere` にある。
