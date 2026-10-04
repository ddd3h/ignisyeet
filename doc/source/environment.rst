環境モデル
==========

大気
----

大気の温度・圧力・密度・音速・粘性係数は、既定では米国標準大気 1976 :cite:`us76` で与える（:numref:`fig-atmosphere`）。
設定 ``[atmosphere]`` で、標準大気に温度差を加えたり、高度によらず一定の大気に置き換えたりできる。
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

.. figure:: _generated/plots/atmosphere.*
   :width: 100%

   米国標準大気 1976 の温度・密度・音速。

温度差（``temperature_offset``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

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
:math:`\Delta T=0` のときは標準大気そのもの（:eq:`eq-us76`）を返す。\ :eq:`eq-us76-offset` を :math:`\Delta T=0` で評価しても、
標準大気の基準圧力 :math:`p_k`\ （表）を相対誤差 :math:`2\times10^{-5}` 以内で再現する。
:math:`\Delta T` は :math:`-200` K より大きくなければならない。

海面の圧力が変わらないので、:math:`\Delta T=+15` K では海面の密度が標準より約 5 %\ （:math:`288.15/303.15` 倍）小さくなる。
空気が薄くなると抗力が減って頂点が高くなり、音速が大きくなるので同じ速度でも Mach 数が下がる。
``[atmosphere]`` の設定は摩擦抗力の Reynolds 数（射点高度）にも使われる。

.. _fig-atmosphere-offset:

.. figure:: _generated/plots/atmosphere_offset.*
   :width: 100%

   温度差 :math:`\Delta T=\pm15` K を加えた大気。左が温度、右が標準大気に対する密度の差。

一定大気（``model = "constant"``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

密度 :math:`\rho`、音速 :math:`a`、粘性係数 :math:`\mu` を高度によらず一定とする（既定値は海面の標準値 1.225 kg/m\ :sup:`3`,
340.29 m/s, 1.789×10\ :sup:`-5` Pa s）。他のコードとの比較や、大気の影響を切り離したい検証に使う。
温度と圧力は、これらと矛盾しないように :math:`T=a^2/(\gamma R)`、:math:`p=\rho R T` として与える。

.. _sec-gravity:

重力
----

重力のモデルは ``[earth]`` の ``gravity`` で選ぶ。

``constant``
   大きさ :math:`g_0=9.80665\ \mathrm{m/s^2}` 一定で、下向き（:math:`-\bm{e}_U`）に働く。

``inverse_square``\ （既定）
   大きさを万有引力の法則に従って高度とともに減らす。

   .. math::
      :label: eq-gravity

      \bm{g}(z_U) = -g_0\left(\frac{R_\oplus}{R_\oplus+h_0+z_U}\right)^2\bm{e}_U,\qquad R_\oplus=6\,371\ \mathrm{km}

   :math:`h_0` は射点標高である。平面地球モードでは向きが ENU の :math:`-z_U` 方向、ECEF モードでは現在位置の測地的な下向き :math:`-\bm{e}_U` で、
   大きさは同じ法則（:math:`z_U` は射点標高からの高さ）で与える。

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
   鉛直方向以外にも小さな成分（赤道向きに約 :math:`3J_2g\sin\phi\cos\phi`）を持つ。
   平面地球モードでは地球の形が定義できないので、``gravity = "j2"`` はエラーになる。

:numref:`fig-gravity` のように、逆 2 乗則では高度 100 km で地表の約 97 %、300 km で約 91 % になる。
数 km 程度の飛翔では差は 0.1 % 程度で、風や空力係数の不確かさに比べて小さい。
しかし、計算コストはほとんど変わらないので、高い高度に到達するロケットにもそのまま使えるよう既定では逆 2 乗則を使う。
:math:`J_2` モデルの地表での大きさは、赤道で約 9.814 m/s\ :sup:`2`、極で約 9.832 m/s\ :sup:`2` である（:ref:`sec-verify-env`）。

.. _fig-gravity:

.. figure:: _generated/plots/gravity.*
   :width: 80%

   高度による重力加速度の変化（逆 2 乗則）。

.. _sec-wind:

風
--

風は軌道と着地点を大きく変える外乱である。風速は高度とともに変わるので、高度分布のモデルが必要になる。
風モデルは ``[wind]`` の ``model`` で選ぶ。いずれも風は水平方向だけで、鉛直成分は 0 である。
:math:`h` は **地上高度**\ （射点標高からの高さ :math:`z_U`、:math:`h<0` では 0 として扱う）である。

風速の高度分布
~~~~~~~~~~~~~~

``power``\ （べき法則、既定）
   地表付近（大気境界層）でよく使われる分布で :cite:`counihan,stull`、

   .. math::
      :label: eq-wind

      w(h) = w_{\mathrm{ref}}\left(\frac{h}{h_{\mathrm{ref}}}\right)^{1/n}

   である。:math:`h_{\mathrm{ref}}` は基準高度 ``ref_height``\ （既定 2 m）、:math:`w_{\mathrm{ref}}` はその高度での風速 ``speed``、
   :math:`n` は地表の状態で決まる定数 ``exponent`` である。:math:`n` が小さいほど、高さとともに風が強くなる（:numref:`fig-wind-profile`）。
   目安として、海上や開けた平地では :math:`n\approx7`、郊外で :math:`n\approx4`〜:math:`5`、都市部で :math:`n\approx3` 程度とされる :cite:`counihan`。

``constant``\ （一定風）
   高度によらず :math:`w(h)=w_{\mathrm{ref}}` とする。

``log``\ （対数則）
   中立な大気境界層の対数則 :cite:`stull` で、粗度長 :math:`z_0`\ （``roughness_length``、既定 0.03 m）を使って

   .. math::
      :label: eq-wind-log

      w(h) = w_{\mathrm{ref}}\,\frac{\ln\bigl(\max(h,\,z_0)/z_0\bigr)}{\ln(h_{\mathrm{ref}}/z_0)}

   とする。式の上では :math:`h\le z_0` で 0 になる（実際の地面付近の扱いは :ref:`sec-wind-blend`）。:math:`z_0` は開けた草地で 0.01〜0.05 m、樹木や建物のある地表で 0.3〜1 m 程度とされる :cite:`stull`。
   :math:`0<z_0<h_{\mathrm{ref}}` でなければならない（:numref:`fig-wind-models` 左）。

``profile``\ （高度別の表）
   次項の表から、高度ごとの風を与える。

.. _fig-wind-profile:

.. figure:: _generated/plots/wind_profile.*
   :width: 80%

   べき法則による風速の高度分布（:math:`w_{\mathrm{ref}}=4` m/s）。

べき法則や対数則は本来、高度数百 m 程度までの大気境界層の近似である :cite:`stull`。
これを数 km まで延長すると、:numref:`fig-wind-profile` や :numref:`fig-wind-models` のように上空の風速を過大に見積もりやすい。
到達高度が高い機体の落下分散（特にパラシュート降下）は、この仮定に強く依存することに注意してほしい。
高高度まで飛ぶ機体では、高層観測や数値予報の風を ``profile`` で与えるのが望ましい。

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
:math:`h\ge h_b` の風速は、:eq:`eq-wind` や :eq:`eq-wind-log` のままである。
``ground_blend_height`` は正で ``ref_height`` より小さい値でなければならない。``log`` では :math:`z_0` より大きい必要があり、
設定値が :math:`z_0` 以下なら :math:`\max(h_b,\,2z_0)` に読み替える。``constant`` と ``profile`` には適用しない。
サンプル（``power``、:math:`h_b=1` m）では、この接続による着地点の変化は約 1 cm である。

.. _sec-wind-profile:

高度別の風の表（``profile``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ラジオゾンデや数値気象モデル（GFS, ECMWF など）から取り出した風は、CSV ファイルにして ``wind.profile`` で指定する。
1 行目は次の見出しでなければならない（大文字小文字と前後の空白は無視する）。空行と ``#`` で始まる行は読み飛ばす。

.. code-block:: text

   altitude_m,speed,direction_deg
   0,3.0,270
   100,6.0,270
   300,9.0,285
   600,12.0,300
   1000,13.0,310

各列は、地上高度 [m]（射点標高からの高さ。**厳密に増加** していなければならない）、風速 [m/s]（0 以上）、
風が吹いてくる方位 [deg]（北から時計回り）である。値は有限の数でなければならず、1 行以上のデータが必要である。

中間の高度では、風速と風向を別々に補間するのではなく、風速ベクトルの成分を高度について **線形補間** する。

.. math::
   :label: eq-wind-profile

   \bm{w}_i=-w_i\begin{pmatrix}\sin\psi_i\\ \cos\psi_i\end{pmatrix},\qquad
   \bm{w}(h)=\bm{w}_i+\frac{h-h_i}{h_{i+1}-h_i}\,(\bm{w}_{i+1}-\bm{w}_i)\qquad(h_i\le h<h_{i+1})

風向が大きく違う 2 点の間では、補間した風速が両端より小さくなる（例えば 350° と 10° の同じ風速 :math:`w` の中間は、方位 0° で風速 :math:`w\cos10^\circ` である）。
表の最下端より低いところでは最下端の値、最上端より高いところでは最上端の値を **一定** に保つ（外挿しない）。
:numref:`fig-wind-models` 右は、上の例の成分を補間したものである。

``profile`` では ``speed`` と ``direction_deg`` は使わない。代わりに、表から基準高度 ``ref_height`` での風を求め、
それを基準風速・基準風向として扱う（``summary.json`` などにはこの値が出る。無風なら風向は 0° とする）。

.. _fig-wind-models:

.. figure:: _generated/plots/wind_models.*
   :width: 100%

   風モデル。左は一定風・べき法則・対数則（:math:`w_{\mathrm{ref}}=4` m/s, :math:`h_{\mathrm{ref}}=2` m）、
   右は高度別の表の例（点が表の値、線が成分の線形補間）。

.. _sec-wind-scaling:

落下分散のための基準風の変更
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

落下分散（:doc:`dispersion`）では、基準風速と基準風向だけを変えた風を何度も作る。
``constant``, ``power``, ``log`` では ``speed`` と ``direction_deg`` を差し替えるだけである。
``profile`` では、表の形（高度ごとの風速比と風向のねじれ）を保つために、基準風速を :math:`w_{\mathrm{ref}}\to w'`、
基準風向を :math:`\psi_{\mathrm{ref}}\to\psi'` に変えるとき、

.. math::
   :label: eq-wind-scale

   w_i' = \frac{w'}{w_{\mathrm{ref}}}\,w_i,\qquad
   \psi_i' = (\psi_i+\psi'-\psi_{\mathrm{ref}})\bmod 360^\circ

と、表の **すべての** 風速を同じ比で拡大し、すべての風向を同じ角度だけ回転する。
基準高度で表の風が 0 のときは比が定義できないので、全高度で一様な風速 :math:`w'`、風向 :math:`\psi'` の一定風に置き換える。

風向の定義
~~~~~~~~~~

風向 :math:`\psi_w` は気象の慣例 :cite:`wmo8` に従い、風が **吹いてくる** 方位を北から時計回りに測る（:numref:`fig-wind-dir`）。
例えば :math:`\psi_w=270^\circ` は西風（西から東へ吹く風）である。風速ベクトルは

.. math::
   :label: eq-wind-vec

   \bm{w}(h) = -w(h)\begin{pmatrix}\sin\psi_w\\ \cos\psi_w\\ 0\end{pmatrix}

で、鉛直成分は考えない。

.. _fig-wind-dir:

.. figure:: _generated/tikz/wind_dir.*
   :width: 80%

   風向 :math:`\psi_w` と射方位 :math:`\psi` の定義。どちらも北から時計回りに測る。

風モデルの比較
~~~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 18 44 20 18

   * - ``model``
     - 内容
     - 適する高度
     - 必要なデータ
   * - ``constant``
     - 高度によらず一定の風速
     - 低高度
     - 地上風
   * - ``power``
     - :eq:`eq-wind`
     - 〜1 km
     - 地上風
   * - ``log``
     - :eq:`eq-wind-log`、:math:`z_0` は粗度長
     - 〜100 m
     - 地上風、粗度
   * - ``profile``
     - 高度別の表の成分補間（:eq:`eq-wind-profile`）
     - 表の範囲
     - ラジオゾンデ・数値予報などの風

突風や乱流（時間変動する風）は扱わない（:doc:`limitations`）。
