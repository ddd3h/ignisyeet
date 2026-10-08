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
:math:`\Delta T` は、モデルの範囲で最も低い標準温度 186.946 K に足しても絶対温度が正になるよう、:math:`-186.946` K より大きくなければならない。

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
密度、音速、粘性係数は、有限で正でなければならない（そうでなければ設定の読み込みでエラーにする）。

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

基準風速 ``wind.speed`` は有限で 0 以上、風向 ``wind.direction_deg`` は有限でなければならない。設定の読み込みの検査（:ref:`sec-config-checks`）のほか、風モデルを直接作るライブラリの呼び出しでも、
``error[IY-S039]: wind.speed must be a finite number >= 0, found …`` と ``error[IY-S040]: wind.direction_deg must be a finite number, found …`` のエラーにする。
風の格子とモンテカルロ法がケースごとに風速と風向を与えるときも、有限でない値は ``wind speed and direction must be finite numbers, found … m/s from … deg`` のエラーにする。

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
   :math:`0<z_0<h_{\mathrm{ref}}/2` でなければならない（:numref:`fig-wind-models` 左）。
   満たさない設定は、``wind.roughness_length (…) must be smaller than half of wind.ref_height (…) for the log profile`` というエラーになる
   （設定ファイルの検査と、風モデルを直接作るライブラリの呼び出しのどちらでも同じ）。

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

ラジオゾンデや数値気象モデル（GFS, ECMWF など）から取り出した風は、CSV ファイルにして ``wind.profile`` で指定する。
1 行目は次の見出しでなければならない（大文字小文字と前後の空白は無視する）。空行と ``#`` で始まる行は読み飛ばす。

.. code-block:: text

   altitude_m,speed,direction_deg
   0,3.0,270
   100,6.0,270
   300,9.0,285
   600,12.0,300
   1000,13.0,310

各列は、地上高度 [m]（射点標高からの高さ。0 以上で、**厳密に増加** していなければならない）、風速 [m/s]（0 以上）、
風が吹いてくる方位 [deg]（北から時計回り。0 以上 360 以下）である。値は有限の数でなければならず、1 行以上のデータが必要である。
方位が 0 から 360 の範囲を外れた行は、``wind profile direction must be between 0 and 360 degrees (row "0,1,400")`` の形で、その行を示してエラーにする。
設定の ``wind.direction_deg`` も同じ範囲である。
高度が負の行（射点より低い行）は、``wind profile altitude must be ≥ 0 m above the launch site (row "-30,3,270"): drop levels below the launch site and add a 0 m row`` の形で、その行を示してエラーにする。

予報やゾンデのデータを表にするときは、次のように換算する。

* 高度が海抜の値なら、``launch.altitude`` を引いて射点からの高さにする。引いた高度が負になる行（射点より低い気圧面など）は捨て、射点の高さ（0 m）の行を加える。
  その行には、捨てた行のうち射点に最も近い風か、射点の地上風を入れる。
* 気圧面ごとのデータは、同じ製品のジオポテンシャル高度を高度の列に使う（海抜なら上と同様に ``launch.altitude`` を引く）。
* 風速が knot なら 0.5144 を掛けて m/s にする（1 kn = 0.5144 m/s）。
* 風が東西成分 :math:`u`\ （東向き正）と南北成分 :math:`v`\ （北向き正）で与えられるなら、風速は :math:`\sqrt{u^2+v^2}`、風が吹いてくる方位は :math:`\mathrm{atan2}(-u,\,-v)`\ [deg] を 0° 以上 360° 未満にした値である（:math:`u=-w\sin\psi,\ v=-w\cos\psi` の逆。例えば西風 :math:`u>0,\ v=0` は 270° になる）。
* 高度は厳密に増加する行にする（同じ高度の行や逆順の行はエラーになる）。表は、予想される頂点より上まで続ける。上端より高いところは上端の風で一定になるので、足りないと、飛翔の上部で表の上端の風が続くものとして計算される。

中間の高度では、風速と風向を別々に補間するのではなく、風速ベクトルの成分を高度について **線形補間** する。

.. math::
   :label: eq-wind-profile

   \bm{w}_i=-w_i\begin{pmatrix}\sin\psi_i\\ \cos\psi_i\end{pmatrix},\qquad
   \bm{w}(h)=\bm{w}_i+\frac{h-h_i}{h_{i+1}-h_i}\,(\bm{w}_{i+1}-\bm{w}_i)\qquad(h_i\le h<h_{i+1})

風向が大きく違う 2 点の間では、補間した風速が両端より小さくなる（例えば 350° と 10° の同じ風速 :math:`w` の中間は、方位 0° で風速 :math:`w\cos10^\circ` である）。
表の最下端より低いところでは最下端の値、最上端より高いところでは最上端の値を **一定** に保つ（外挿しない）。
:numref:`fig-wind-models` 右は、上の例の成分を補間したものである。

``profile`` では ``speed`` と ``direction_deg`` は使わない（設定ファイルに書いてあると、使わないことを警告する）。代わりに、表全体の代表風を基準風として扱う（``summary.json`` などにはこの値が出る）。
代表風速 :math:`W` は、表の最下行から最上行までの高度範囲での平均風速

.. math::
   :label: eq-wind-profile-mean

   W=\frac{1}{h_N-h_1}\int_{h_1}^{h_N}\bigl|\bm{w}(h)\bigr|\,\mathrm{d}h

であり（台形則で評価する。1 行だけの表ではその行の風速）、代表風向 :math:`\Psi` は、同じ範囲で平均した風速ベクトル
:math:`\overline{\bm{w}}=\frac{1}{h_N-h_1}\int\bm{w}\,\mathrm{d}h` が吹いてくる方位である。
平均ベクトルがほぼ 0 のとき（互いに逆向きの風が打ち消し合うとき）は、基準高度 ``ref_height`` での風向を使う（そこも無風なら 0°）。
:math:`W` はベクトルの大きさの平均なので、風向が高度で変わる表では平均ベクトルの大きさ :math:`|\overline{\bm{w}}|` より大きい。
:math:`W` は表の全範囲での平均なので、頂点よりずっと高い行まで表が続くと、飛翔に関係しない上空の風が :math:`W` に入る。
表を飛翔の高度範囲に切ると、:math:`W` と、それを基にした風速格子やモンテカルロ法の標準偏差の解釈が容易になる。
切るときは、公称の頂点のすぐ上ではなく、風の格子とモンテカルロ解析で予想される最も高い頂点（たとえば公称の頂点 + 3σ。σ はモンテカルロ解析の頂点高度の標準偏差 ``apogee_std``）より上まで残す。
表より上の風は最上端の値で一定になるので、頂点が最上端を超える飛翔は、その上の風を表していない。頂点が最上端を超えたケースとサンプルは、数と最も高い頂点が警告に出る（:ref:`sec-config-checks`）。
:math:`W` は ``ignisyeet sim`` の ``summary.json`` の ``wind_speed`` に出る。

.. _fig-wind-models:

.. figure:: _generated/plots/wind_models.*
   :width: 100%

   風モデル。左は一定風・べき法則・対数則（:math:`w_{\mathrm{ref}}=4` m/s, :math:`h_{\mathrm{ref}}=2` m）、
   右は高度別の表の例（点が表の値、線が成分の線形補間）。

.. _sec-wind-scaling:

落下分散のための基準風の変更
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

落下分散（:doc:`dispersion`）では、基準風速と基準風向だけを変えた風を何度も作る。
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
ほぼ無風の表（平均風速が 0.1 m/s 未満）は拡大・回転できず、``wind profile is calm (mean speed … m/s): it cannot be scaled to wind.profile_mean_speed / rotated to wind.profile_mean_direction_deg`` のエラーになる。
値の範囲の検査は :ref:`sec-config-checks` にある。

モンテカルロ法の風速の揺らぎ :math:`\delta`\ （``wind_speed``、単位 m/s）は、``profile`` では平均風速 :math:`W` に対する揺らぎであり、
:math:`k=\max(W+\delta,\,0)/W` で表を拡大する。:math:`k\ge0` なので、揺らぎで風向が反転することはない。

表の平均風速が 0.1 m/s 未満（ほぼ無風）の表は拡大できない。風速格子、および風速の揺らぎが 0 でないモンテカルロ法では、
一様な風へ置き換えることはせず、エラーにする。
風速格子では ``wind profile is calm (mean speed … m/s): it cannot be scaled for dispersion; use wind.model = "constant"/"power" for a wind grid`` となり、``model = "constant"`` または ``"power"`` で風速格子を作る。
モンテカルロ法では ``wind profile is calm (mean speed … m/s): the Monte Carlo wind perturbations cannot be applied to it; set dispersion.monte_carlo.wind_speed and wind_direction_deg to 0, or use a profile with wind`` となり、風速と風向の標準偏差を 0 にするか、風のある表を使う。
単一の飛翔（``ignisyeet sim``）は拡大しないので、ほぼ無風の表でも実行できる。

拡大率 :math:`k` が 3 を超える、または 1/3 を下回るケースがあると、警告を出す。
表の風は上空ほど強いのが普通で、拡大は上空の風速にも同じ比で効くので、:math:`k` が大きいと上空の風が非現実的に強くなるからである。
``summary.json`` の ``wind_scale`` は、その飛翔で使った :math:`k`\ （CSV の表そのままなら 1）である。``profile`` 以外のモデルでは出力しない。
表の最下行が地上高度 0 m より上にあるときは、それより下の風は最下行の値に保たれる（警告を出す）。

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
