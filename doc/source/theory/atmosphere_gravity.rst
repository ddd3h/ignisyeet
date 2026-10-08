.. _page-theory-atmosphere-gravity:

大気と重力
==========

大気は空気力と音速（Mach 数）を、重力は軌道そのものを決める。どちらも既定のままで多くの飛翔に足りるが、
気温が標準から大きく外れる日、高い頂点まで飛ぶ機体、他のコードとの比較では選択が結果に効く。

この章で決めること
--------------------------

.. list-table::
   :header-rows: 1
   :widths: 36 64

   * - 設定キー
     - 決めること
   * - :ref:`cfg-atmosphere-model`
     - 標準大気（``us1976``）か、高度によらない一定大気（``constant``）か
   * - :ref:`cfg-atmosphere-temperature_offset`
     - 標準大気に加える一定の温度差 [K]（``us1976`` のみ）
   * - :ref:`cfg-atmosphere-density`、:ref:`cfg-atmosphere-sound_speed`、:ref:`cfg-atmosphere-viscosity`
     - 一定大気の密度・音速・粘性係数（``constant`` のみ）
   * - :ref:`cfg-earth-gravity`
     - 重力の大きさの高度変化：``constant``、``inverse_square``、``j2``

選び方
------

.. _tbl-choose-atmosphere:

.. list-table:: 大気モデルの選び方
   :header-rows: 1
   :widths: 14 20 18 18 30

   * - 選択肢
     - モデルが表すもの
     - 仮定・有効範囲
     - 計算コスト
     - 選ぶとき・避けるとき
   * - ``us1976``、``temperature_offset = 0``\ （既定）
     - 米国標準大気 1976（86 km まで）
     - 気温・気圧が標準の日。精度の根拠は文献値との一致（:ref:`sec-verify-env`）
     - 低い（層ごとの式）
     - 気温の実測がない、または標準的な気温の日に選ぶ
   * - ``us1976`` に ``temperature_offset`` を加える
     - 標準大気の温度に高度によらない一定の温度差 :math:`\Delta T` を加えた大気
     - 海面気圧は標準の 101 325 Pa のまま。逆転層・湿度は考えない。:math:`\Delta T>-186.946` K
     - ``us1976`` と同じ
     - 打上げ日の気温が標準（15 °C）と大きく違うとき、季節差を見るとき。気圧が標準から大きく外れる日には使えない
   * - ``constant``
     - 密度・音速・粘性係数を高度によらず一定にした大気
     - 頂点が低く、大気の高度変化が結果に効かないと見なせる検証用
     - 最も低い
     - 風洞条件や他のコードとの比較で、大気の影響を切り離したいとき。実際の飛翔の予測には選ばない

大気の選択が結果に効く大きさを :numref:`tbl-atmosphere-effect` に示す。

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

.. _tbl-choose-gravity:

.. list-table:: 重力モデルの選び方
   :header-rows: 1
   :widths: 16 22 20 18 24

   * - 選択肢
     - モデルが表すもの
     - 仮定・有効範囲
     - 精度の根拠
     - 選ぶとき・避けるとき
   * - ``constant``
     - 大きさ 9.80665 m/s\ :sup:`2` 一定の有効重力
     - 頂点が低い（数 km）飛翔
     - 頂点 4.4 km のサンプルで ``inverse_square`` と頂点 1.7 m、着地点 6 m 差
     - 低高度の検証、解析解との比較。高い頂点には避ける
   * - ``inverse_square``\ （既定）
     - 距離の 2 乗に反比例して減る有効重力
     - 球対称の地球。緯度による変化なし
     - 100 km で地表の約 97 %、300 km で約 91 %
     - 平面地球・ECEF のどちらでも使える。迷ったらこれ
   * - ``j2``
     - 点質量に扁平（:math:`J_2`）を加えた万有引力
     - ECEF モードのみ。平面地球ではエラー
     - 地表で赤道 9.814、極 9.832 m/s\ :sup:`2`\ （:ref:`sec-verify-env`）
     - ECEF モードで緯度による重力差まで含めたいとき

重力の選択が結果に効く大きさを :numref:`tbl-gravity-effect` に示す。サンプルの飛翔（頂点 4.4 km、着地まで約 7.7 km）では、どの選択でも着地点の差は 10 m 以内である。

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

モデル
------

大気
~~~~

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

.. _sec-gravity:

重力
~~~~

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

適用範囲と限界
--------------

* 標準大気の温度差は、高度によらず一定の値を加えるだけである。逆転層や湿度は考えない。気圧の日変化も入らない（海面気圧は 101 325 Pa に固定）。
* 摩擦抗力の Reynolds 数は射点高度の大気で固定しており、高度による変化を含まない（:ref:`sec-lim-aero`）。
* 大気の設定を変えると、係数表も作り直される。外部の表（``method = "table"``）は更新されない（:ref:`sec-lim-env`）。
* 重力は点質量に :math:`J_2` 項を加えたところまでで、高次の項や重力異常は考えない :cite:`hofmann`。``constant`` と ``inverse_square`` は緯度による変化を含まない（:ref:`sec-lim-earth`）。
* ``j2`` は ECEF モードでのみ使える。

精度の証拠
----------

* 標準大気は、海面で :math:`\rho=1.225\ \mathrm{kg/m^3}`、:math:`a=340.29\ \mathrm{m/s}`、11 km で :math:`T=216.77\ \mathrm{K}`、20 km で :math:`\rho=0.0889\ \mathrm{kg/m^3}` となることを、文献値 :cite:`us76` と照らして確かめている（検証。:ref:`sec-verify-env`）。
* 温度差は、:math:`\Delta T=0` で標準大気と一致し、:math:`\Delta T=15` K の静水圧平衡が微分方程式の数値積分と相対誤差 :math:`10^{-3}` 以内で一致することを確かめている（検証。:ref:`sec-verify-env`）。
* :math:`J_2` 重力は、地表の赤道と極の大きさを確かめている（検証。:ref:`sec-verify-env`）。
* 大気と重力を実測と比べた妥当性確認はない。

関連する設定キー
----------------

.. list-table::
   :header-rows: 1
   :widths: 38 20 42

   * - キー
     - 既定値
     - 効果の向き
   * - :ref:`cfg-atmosphere-model`
     - ``us1976``
     - ``constant`` にすると上空でも密度が高く、頂点が低くなる
   * - :ref:`cfg-atmosphere-temperature_offset`
     - 0 K
     - 正で空気が薄くなり、頂点が高くなる（サンプルで +15 K あたり約 +3 %）
   * - :ref:`cfg-atmosphere-density`
     - 1.225 kg/m\ :sup:`3`
     - 一定大気だけで使う
   * - :ref:`cfg-atmosphere-sound_speed`
     - 340.29 m/s
     - 一定大気だけで使う。大きいと同じ速度で Mach 数が下がる
   * - :ref:`cfg-atmosphere-viscosity`
     - 1.789×10\ :sup:`-5` Pa s
     - 一定大気だけで使う。摩擦抗力の Reynolds 数に効く
   * - :ref:`cfg-earth-gravity`
     - ``inverse_square``
     - ``j2`` は ECEF モードのみ。着地点の差は数 m（サンプル）

値の完全な定義は :ref:`page-reference-config-environment` にある。
