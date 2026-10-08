.. _page-theory-wind:

風
==

風は軌道と着地点を大きく変える外乱である。風速は高度とともに変わるので、高度分布のモデルが必要になる。
同じ地上風でも、高度分布の仮定を変えるだけで着地距離は数倍変わる（:numref:`tbl-wind-effect`）。
風モデルの選択は、この章で決める最も影響の大きい選択である。

この章で決めること
--------------------------

.. list-table::
   :header-rows: 1
   :widths: 36 64

   * - 設定キー
     - 決めること
   * - :ref:`cfg-wind-model`
     - 高度分布：``constant``、``power``、``log``、``profile``
   * - :ref:`cfg-wind-speed`、:ref:`cfg-wind-direction_deg`
     - 基準高度での風速 [m/s] と風向 [deg]（``profile`` では使わない）
   * - :ref:`cfg-wind-ref_height`
     - 基準風速を測った高度 [m]（既定 2 m）
   * - :ref:`cfg-wind-exponent`
     - ``power`` の指数 :math:`n`\ （地表の状態）
   * - :ref:`cfg-wind-roughness_length`
     - ``log`` の粗度長 :math:`z_0` [m]
   * - :ref:`cfg-wind-ground_blend_height`
     - 地面付近のなめらかな接続の高さ [m]
   * - :ref:`cfg-wind-profile`
     - ``profile`` の高度別の風の表
   * - :ref:`cfg-wind-profile_mean_speed`、:ref:`cfg-wind-profile_mean_direction_deg`
     - ``profile`` の平均風の目標（表を拡大・回転する）

選び方
------

.. _fig-choose-wind-model:

.. figure:: /_generated/tikz/wind_model_choice.*
   :width: 80%

   風モデルの選び方。上から順に答えると、観測または予報があれば ``profile``、なければ頂点の高さと粗度長の有無で ``power`` か ``log`` に至る。橙の箱は上空へ外挿する選択で、結果は参考値にとどまる。

.. _tbl-choose-wind:

.. list-table:: 風モデルの比較
   :header-rows: 1
   :widths: 12 30 22 36

   * - 選択肢
     - 表すもの・仮定
     - 適する高度・必要なデータ
     - 選ぶとき・避けるとき
   * - ``constant``
     - 高度によらず一定の風速。式 :math:`w(h)=w_{\mathrm{ref}}`
     - 低高度。地上風
     - 他のコードや解析解との比較、風の影響の切り分け。上空の風が地上風より強いのが普通なので、実際の予測には避ける
   * - ``power``
     - 大気境界層のべき法則 :eq:`eq-wind`。指数 :math:`n` で地表の状態を表す
     - 〜1 km。地上風と指数
     - 観測がなく、頂点が 1 km 程度までのとき。上空へ延長すると風速を過大に見積もりやすい
   * - ``log``
     - 中立な大気境界層の対数則 :eq:`eq-wind-log`。粗度長 :math:`z_0` で地表の状態を表す
     - 〜100 m（実用は 1 km 程度まで）。地上風と粗度長
     - 粗度長が分かり、低い飛翔のとき。:math:`0<z_0<h_{\mathrm{ref}}/2` が必要
   * - ``profile``
     - 高度別の風の表（風速ベクトルの線形補間 :eq:`eq-wind-profile`）
     - 表の範囲。ゾンデ・数値予報の風
     - 観測や予報があるとき。頂点が 1 km を超える飛翔、パラシュート降下の落下分散。表は予想される最高の頂点より上まで続ける

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

基準高度の決め方
~~~~~~~~~~~~~~~~

``wind.speed`` は **基準高度 ``wind.ref_height`` での風速** である（既定の高度は 2 m）。
風速計を 10 m に立てて測ったなら、``ref_height = 10`` と測定値を ``speed`` に与える。
同じ測定値でも ``ref_height`` が違えば、上空の風速は変わる。
``power`` と ``log`` は基準高度の風を上空へ延ばす。10 m で測った風速を ``ref_height = 2`` のまま ``speed`` に入れると、上空の風を約 3 割過大に見積もる
（:math:`n=6` のとき :math:`(10/2)^{1/6}\approx1.31`）。
``profile`` の表では、``wind.ref_height`` での表の風が ``summary.json`` の ``wind_at_ref_height`` に出る。

風モデルが結果に効く大きさ
~~~~~~~~~~~~~~~~~~~~~~~~~~

:numref:`tbl-wind-ratio` は、基準風速に対する各高度の風速の比である。同じ地上風でも、モデルと指数で上空の風速が大きく違う。

.. _tbl-wind-ratio:

.. list-table:: 基準高度（2 m）の風速に対する風速の比 :math:`w(h)/w_{\mathrm{ref}}`\ （式 :eq:`eq-wind`、:eq:`eq-wind-log` による）
   :header-rows: 1
   :widths: 34 16 16 16 18

   * - モデル
     - 100 m
     - 500 m
     - 1000 m
     - 4000 m
   * - ``power``、:math:`n=4`
     - 2.66
     - 3.98
     - 4.73
     - 6.69
   * - ``power``、:math:`n=6`
     - 1.92
     - 2.51
     - 2.82
     - 3.55
   * - ``power``、:math:`n=7`
     - 1.75
     - 2.20
     - 2.43
     - 2.96
   * - ``log``、:math:`z_0=0.03` m
     - 1.93
     - 2.31
     - 2.48
     - 2.81
   * - ``constant``
     - 1
     - 1
     - 1
     - 1

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

基準風の拡大
~~~~~~~~~~~~

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

:numref:`tbl-wind-profile-scale` は、``examples/wind_profile_example.csv``\ （平均風速 :math:`W=11.89` m/s）を ``profile_mean_speed`` で拡大したときの倍率と着地点である（サンプル機体、``sim``。データは ``doc/data/sim/wind_keys.csv``）。
風速格子の ``wind_speeds`` の値も、``profile`` ではこの平均風速 :math:`W'` として同じ倍率 :math:`k=W'/W` で表に掛かる。
着地の北向きの距離は、拡大率にほぼ比例して増える。頂点はほとんど変わらない。

.. _tbl-wind-profile-scale:

.. list-table:: ``profile_mean_speed`` と着地点（``wind_profile_example.csv``、:math:`W=11.89` m/s）
   :header-rows: 1
   :widths: 24 16 20 20 20

   * - ``profile_mean_speed`` [m/s]
     - 倍率 :math:`k`
     - 頂点 [m]
     - 着地点 東 [m]
     - 着地点 北 [m]
   * - 3
     - 0.25
     - 4479.1
     - :math:`-565.2`
     - 1107.7
   * - 6
     - 0.50
     - 4479.6
     - :math:`-465.7`
     - 2215.2
   * - 9
     - 0.76
     - 4479.3
     - :math:`-366.5`
     - 3321.6
   * - 11.89（指定なし）
     - 1.00
     - 4478.3
     - :math:`-272.4`
     - 4384.1
   * - 15
     - 1.26
     - 4476.4
     - :math:`-173.4`
     - 5528.8

``ground_blend_height``\ （``power`` と ``log`` の地面近くのなめらかな接続の高さ）は、既定の 1 m から 0.25〜1.5 m に変えても、サンプルの着地点は 0.03 m しか動かない（``doc/data/sim/wind_keys.csv``）。
値を変える理由は通常ない。変えるのは、設定値が ``ref_height`` 以上か ``log`` の粗度長以下になってエラーや読み替えが出るときだけである。

``profile`` で最も遠くなる風を機体のばらつきと合わせて調べたいときは、平均風の目標 ``profile_mean_speed`` と ``profile_mean_direction_deg`` で表を拡大・回転して公称の風にする（:ref:`sec-wind-scaling`）。

モデル
------

.. _sec-wind:

風モデル
~~~~~~~~

風モデルは ``[wind]`` の ``model`` で選ぶ。いずれも風は水平方向だけで、鉛直成分は 0 である。
:math:`h` は **地上高度**\ （射点標高からの高さ :math:`z_U`、:math:`h<0` では 0 として扱う）である。
基準風速 ``wind.speed`` は有限で 0 以上、風向 ``wind.direction_deg`` は有限でなければならない。

風速の高度分布
~~~~~~~~~~~~~~

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

適用範囲と限界
--------------

* 風は水平方向だけで、時間によらない定常な風である。突風や乱流は考えない。
* ``power`` と ``log`` は大気境界層（数百 m まで）の近似で、上空の風速を過大に見積もりやすい。頂点が地上 1000 m を超えると、これらのモデルが上空へ外挿されることを警告する（:ref:`IY-O514`）が、結果は修正しない。
* ``profile`` の表の範囲外は端の値で一定とし、時刻や水平位置による変化は表現できない。
* ECEF モードの風は、機体の現在位置の局所 ENU で与える。射点の基底との差は射点からの距離を地球半径で割った角度（55 km で約 0.5°）で、風の不確かさに比べて無視できる。
* 風の格子とモンテカルロ法の拡大は、表の形を保つ（風速比と風向のねじれを同じ割合で拡大・回転する）近似であり、風の鉛直構造の変化は表現しない。

詳しい一覧は :ref:`sec-lim-env` にある。

精度の証拠
----------

次を検証している（モデルと解析解・定義との比較。:ref:`sec-verify-env`）。

* ``power`` と ``constant``：基準高度で :math:`w_{\mathrm{ref}}` に一致し、128 m（基準高度の 64 倍）では 1.9 倍を超える。
* ``log``：:math:`h_{\mathrm{ref}}=10` m、:math:`z_0=0.1` m、:math:`w_{\mathrm{ref}}=4` m/s で、基準高度で 4 m/s、:math:`h\le z_0` で 0、100 m で :math:`4\ln1000/\ln100=6` m/s となる。
* ``profile``：成分の線形補間、端の値の保持、350° と 10° の中間が方位 0°・風速 :math:`4\cos10^\circ` m/s になること、代表風の定義、基準風の拡大・回転（:eq:`eq-wind-scale`）。
* 平均風の目標：``profile_mean_speed = 15.6``、``profile_mean_direction_deg = 123`` の表は、風速と風向がその値になり、同じ風速・風向の風の格子のケースの風とビット単位で一致する。
* 地面付近の接続：対数則を :math:`h_{\mathrm{ref}}=10` m、:math:`z_0=0.1` m、接続の高さ 0.11 m とすると、補正のない 3 次の Hermite 曲線は 0.055 m で風速が :math:`-0.142` m/s まで下がるが、補正した曲線は風速が 0 以上かつ単調に増える。

風モデルを実測の風と比べた妥当性確認はない。べき法則と対数則が実際の上空の風をどこまで表すかは、風の観測で別に確かめる必要がある（:ref:`sec-lim-env`）。

関連する設定キー
----------------

.. list-table::
   :header-rows: 1
   :widths: 38 20 42

   * - キー
     - 既定値
     - 効果の向き
   * - :ref:`cfg-wind-model`
     - ``power``
     - 上空へ外挿するモデルほど着地距離が大きく出る傾向（:numref:`tbl-wind-effect`）
   * - :ref:`cfg-wind-speed`
     - 4.0 m/s
     - 大きいほど着地距離が増える
   * - :ref:`cfg-wind-direction_deg`
     - 0°（北から）
     - 風が吹いてくる方位
   * - :ref:`cfg-wind-ref_height`
     - 2 m
     - 基準風速を測った高度。高いほど、同じ値で上空の風が強くなる
   * - :ref:`cfg-wind-exponent`
     - 6.0
     - 小さいほど上空の風が強い（:numref:`tbl-wind-ratio`）
   * - :ref:`cfg-wind-roughness_length`
     - 0.03 m
     - 大きいほど地表付近の風の立ち上がりが緩やか
   * - :ref:`cfg-wind-ground_blend_height`
     - 1 m
     - 地面付近の接続の高さ。着地点への影響は約 1 cm（サンプル）
   * - :ref:`cfg-wind-profile`
     - なし
     - ``profile`` で風の表（CSV）を指定する
   * - :ref:`cfg-wind-profile_mean_speed`、:ref:`cfg-wind-profile_mean_direction_deg`
     - 表のまま
     - 表の平均風を目標の値へ拡大・回転する

値の完全な定義は :ref:`page-reference-config-environment` にある。
