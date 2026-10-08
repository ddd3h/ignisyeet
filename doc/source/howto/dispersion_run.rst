.. _page-howto-dispersion-run:

落下分散を実行する
====================

風の条件で着地点がどれだけ広がるかを ``ignisyeet dispersion`` で計算し、結果を読む手順を示す。
落下分散には、風だけを変える **風の格子**\ （``mode = "wind_grid"``、既定）と、推力・質量・空力係数・射角・風などのばらつきを同時に与える **モンテカルロ** 解析（``mode = "monte_carlo"``）がある。
このページを終えると、どちらも実行し、着地しなかったケースや不安定なケースを見分けて、結果を読める。
保安距離（保安域の半径）を求める手順は :ref:`page-howto-keepout-zone` にある。どちらを選ぶかの判断は :ref:`page-theory-dispersion` にある。

前提
------

* ``ignisyeet sim`` が設定ファイルで正常に終わる（終了ステータス 0。:ref:`page-howto-read-results`）。
* 風の設定が決まっている（:ref:`page-howto-environment`）。高度別の表（``wind.model = "profile"``）を使うときは、頂点より上まで続く表を用意する。
* 回収系の設定が済んでいる（:ref:`page-howto-launcher-parachute`）。パラシュートが有効なら、弾道落下（開傘しない場合）とパラシュート降下の両方が計算される。
* この手順の例は ``examples/sample.toml``（射角 85 deg、射方位 270 deg、北風 4 m/s、風のモデルは ``power``）である。

手順
------

1. 風の格子を実行する
~~~~~~~~~~~~~~~~~~~~~~~~

設定の ``[dispersion]`` に、基準風速の一覧 ``wind_speeds`` と方位の数 ``directions`` を書く。

.. code-block:: toml

   [dispersion]
   mode = "wind_grid"
   wind_speeds = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]   # [wind] の基準高さでの風速 [m/s]
   directions = 8                                      # 北から等間隔の方位の数

``wind_speeds`` の意味は風モデルで異なる。

* ``constant``、``power``、``log``: ``wind.ref_height`` での風速。べき法則と対数則はこの風速を上空へ外挿するので、頂点が地上 1000 m を超えると警告が出る。
* ``profile``: 表の **平均風速** :math:`W`\ （地上の風速ではない）。表の形を保ったまま、全体を :math:`v/W` 倍に拡大する。地上風の測定値をそのまま入れてはならない。
  平均風速がほぼ 0 の表は拡大できないのでエラーになる。

ケース数は、風速の数、方位の数、降下モードの数（弾道落下とパラシュート降下の 2 つ）の積である。上の設定は :math:`7\times8\times2=112` ケースである。

.. code-block:: console

   $ ignisyeet dispersion examples/sample.toml

期待する結果:

.. code-block:: text

   Dispersion: 112 cases in 997.30ms -> out/dispersion.csv, out/dispersion_meta.json
     Ballistic: max landing distance 1549.6 m
     Parachute: max landing distance 14315.6 m

``out/dispersion.csv``、``out/dispersion_meta.json`` に加えて、``[output] kml = true`` なら ``out/ignisyeet.kml`` が書かれる。

.. _sec-wind-grid-result:

2. 風の格子の結果を読む
~~~~~~~~~~~~~~~~~~~~~~~~~~

:numref:`fig-dispersion` は、同じ風速で風向だけを変えた着地点を線で結んだものである。内側から外側へ、風速が大きくなる。

.. _fig-dispersion:

.. figure:: /_generated/plots/dispersion.*
   :width: 100%

   サンプル機体の落下分散（射角 85°、射方位 270°、7 風速 × 8 方位）。左が弾道落下、右がパラシュート降下。多角形の中心の位置と、風速に応じた広がりの向きを見る。

弾道落下（左）
   機体は風見効果で風上を向いて飛ぶので、着地点は **風上側** にずれる。
   多角形の中心が射点の西（射方位）にずれているのは、射角 85° で西へ打ち出しているためである。

パラシュート降下（右）
   機体は風に流されるので、着地点は **風下側** にずれる。降下時間が長い（頂点 4.4 km から約 10 分）ので、ずれは弾道落下より 1 桁大きい。
   7 m/s で 14 km を超えるが、これはべき法則（:math:`n=6`）が上空の風速を大きく見積もるためでもある（:ref:`page-theory-wind`）。

見るときの注意は次のとおりである。

* **最大着地距離は、計算した条件の中の最大値である。** 風速と方位の刻みを変えると値が変わりうる。サンプルの最大着地距離は、8、36、72 方位で同じ（パラシュート降下 14315.6 m、弾道落下 1549.6 m。どちらも最大が東西の風向にあり、どの方位数にも含まれるため）だが、一般には変わる。
* **多角形は近似である。** 多角形（と KML の輪郭）は、隣り合う方位の着地点を直線で結んだものなので、方位が粗いと着地点が線の外に出る。サンプルの風速 7 m/s で方位を 8 から 72 に増やすと、8 方位の多角形の外側に着地点が出る。その多角形からの距離は最大で、パラシュート降下が約 1050 m（風向 70° と 110°）、弾道落下が約 48 m（風向 65° と 115°）である。
  保安域の判断には、多角形ではなく最大着地距離を半径とする円か、方位を 36 以上にした外側の包絡を使い、格子を細かくして最大値を確かめる。
* **弾道落下にはいびつな点が出ることがある。** 頂点付近でほぼ真上を向いていると、頂点で失速して回転し、落ちていく向きがわずかな条件の差で変わるためである。計算の誤りではない。安全の評価では、このような点を含めた最も外側の範囲で考える。

3. モンテカルロ解析を実行する
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

設定の ``[dispersion]`` の ``mode`` を ``"monte_carlo"`` にし、``[dispersion.monte_carlo]`` にサンプル数 ``samples`` と乱数の種 ``seed`` を書く。各ばらつきの標準偏差は同じ表に書き、0 にするとそのばらつきは無効になる。

.. code-block:: toml

   [sim]
   integrator = "rk45"

   [dispersion]
   mode = "monte_carlo"

   [dispersion.monte_carlo]
   samples = 1000
   seed = 1
   # truncate_sigma = 3.0   # 指定すると、標準偏差の 3 倍を超えるばらつきを引かない

.. code-block:: console

   $ ignisyeet dispersion examples/sample.toml --set sim.integrator=rk45 --set dispersion.mode=monte_carlo

ばらつきの項目、単位、既定の標準偏差は :ref:`tbl-mc-perturb` にある。標準偏差は利用者が決める値で、既定値は目安にすぎない。
同じ設定なら、同じ ``seed`` で毎回同じ結果になる。
サンプル数の目安は、計算の目的で決まる（:ref:`page-theory-dispersion`）。保安距離の検討では :math:`10^4` 以上にする（:ref:`page-howto-keepout-zone`）。

期待する結果（``seed = 1``、1000 サンプル、20 スレッドのマシンで 1.1 s）:

.. code-block:: text

   Monte Carlo: 1000 samples x 2 descent modes in 1.12s -> out/dispersion_mc.csv, out/dispersion_summary.json
     Ballistic: 1000 ok, 0 failed or did not land
       mean landing E -1080.3 m, N 369.6 m; apogee 4480.0 +- 487.4 m
       1-sigma ellipse 237.0 x 149.3 m, 3-sigma 710.9 x 448.0 m, major axis bearing 131 deg
       max landing distance 2141.5 m (sample 935)
       landing distance P50 1131.1 m, P90 1458.9 m, P99 1848.3 m, P99.9 1999.5 m (landed samples)
     Parachute: 1000 ok, 0 failed or did not land
       mean landing E -651.7 m, N -7663.4 m; apogee 4480.0 +- 487.4 m
       1-sigma ellipse 2185.1 x 2066.8 m, 3-sigma 6555.4 x 6200.3 m, major axis bearing 16 deg
       max landing distance 18018.1 m (sample 830)
       landing distance P50 7835.0 m, P90 10907.1 m, P99 13666.4 m, P99.9 14999.4 m (landed samples)

4. モンテカルロ解析の結果を読む
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

結果は :numref:`fig-dispersion-mc` と :numref:`tbl-mc-result` に示す。図と表は ``examples/out/mc`` の結果（上の設定）から作った。

.. _fig-dispersion-mc:

.. figure:: /_generated/plots/dispersion_mc.*
   :width: 100%

   モンテカルロ解析による着地点の分布。点が各サンプルの着地点、十字が平均、実線と破線が :math:`1\sigma`、:math:`3\sigma` の誤差楕円、星が射点。左が弾道落下、右がパラシュート降下（どちらも 1000 点）。点の広がりと楕円の向きを見る。

.. _tbl-mc-result:

.. list-table:: モンテカルロ解析の結果（サンプル、1000 サンプル、``seed = 1``）
   :header-rows: 1
   :widths: 40 30 30

   * - 量
     - 弾道落下
     - パラシュート降下
   * - 有効サンプル数 / 失敗
     - 1000 / 0
     - 1000 / 0
   * - 着地点の平均 東 / 北 [m]
     - −1080.3 / 369.6
     - −651.7 / −7663.4
   * - :math:`1\sigma` 楕円 長半径 × 短半径 [m]
     - 237.0 × 149.3
     - 2185.1 × 2066.8
   * - :math:`3\sigma` 楕円 長半径 × 短半径 [m]
     - 710.9 × 448.0
     - 6555.4 × 6200.3
   * - 長軸の方位 [deg]
     - 131
     - 16
   * - 着地距離 P50 / P90 / P99 / P99.9 [m]
     - 1131.1 / 1458.9 / 1848.3 / 1999.5
     - 7835.0 / 10907.1 / 13666.4 / 14999.4
   * - 最大着地距離 [m]（サンプル番号）
     - 2141.5（935）
     - 18018.1（830）
   * - 頂点高度の平均 ± 標準偏差 [m]
     - 4480.0 ± 487.4
     - 4480.0 ± 487.4

読み方は次のとおりである。

* **平均と楕円。** 弾道落下の平均着地点は、射点から西に 1080 m、北に 370 m にある。分布はこの平均の周りで、長軸の方位 131°（北から時計回り。南東と北西を結ぶ向き）にやや細長く広がる。平均の方向と長軸の向きは別の量である。
  パラシュート降下の広がりは約 2 km（:math:`1\sigma`）で、弾道落下より 1 桁大きい。長半径と短半径がほぼ等しい（2185 m と 2067 m）ので、長軸の方位はあまり意味を持たない。
  楕円の定義は :ref:`fig-error-ellipse` にある。
* **楕円の内側の割合。** 正規分布なら :math:`1\sigma` 楕円の内側に約 39 %、:math:`3\sigma` 楕円の内側に約 99 % が入る。この計算では、弾道落下で 44 % と 98 %、パラシュート降下で 41 % と 99 % で、分布は正規分布から少しずれている。
* **裾はパーセンタイルと最大値で見る。** 楕円は正規分布を仮定した要約である。保安域の半径を考えるときは、楕円ではなく P90、P99、P99.9 と最大着地距離を使う（:ref:`sec-mc-percentile`）。
  P99.9 は着地したサンプルが 1000 以上のときだけ出る。1000 未満では ``P99.9 n/a (needs ≥ 1000 landed samples)`` と表示され、``dispersion_summary.json`` の ``landing_distance_p999`` は ``null`` になる。
  この計算の P99.9 は、1000 サンプルでは上位 2 個ほどの点で決まる粗い値である。最大値はサンプル数とともに大きくなる（:ref:`tbl-safety-samples`）。
* **頂点高度の平均。** 4480 m は公称の飛翔（4440 m）より 40 m ほど高い。ばらつきが平均について対称でも、頂点高度は推力や抗力係数に対して線形ではないので、平均は公称値と一致しない。

出力ファイルの列と項目は :ref:`page-reference-output-files` にある。

5. Google Earth で見る
~~~~~~~~~~~~~~~~~~~~~~~~

``[output] kml`` が ``true``\ （既定）のとき、``ignisyeet dispersion`` は ``ignisyeet.kml`` を書く。Google Earth で開く（ファイルをウィンドウにドラッグするか、「ファイル」メニューの「開く」で選ぶ）と、射点を中心に地形の上へ表示される。同じ出力先に ``sim`` の結果があれば、飛翔経路もフォルダ ``Flight`` に入る。

フォルダ ``Dispersion – wind grid``\ （``mode = "wind_grid"``）
   降下モードごとのフォルダの下に風速ごとのフォルダがあり、チェックボックスで表示を切り替えられる。風速ごとに、着地点を風向の順に結んだ輪郭と、各ケースの着地点の印が入る。印をクリックすると、風速・風向、着地点の東・北方向の距離、着地時刻と着地速度、頂点高度が見られる。

フォルダ ``Dispersion – Monte Carlo``\ （``mode = "monte_carlo"``）
   降下モードごとに、平均着地点、:math:`1\sigma` と :math:`3\sigma` の誤差楕円、各サンプルの着地点が入る。サンプル数が多いときは、フォルダ ``Landing points`` の表示を切ると楕円だけを見られる。失敗したサンプルと着地しなかったサンプルは含まれない。

風の格子の輪郭は着地点を直線で結んだものなので、保安域の確認には輪郭だけでなく着地点の印も見る。着地しなかったケースは輪郭と着地点から除かれる。
``sim`` と ``dispersion`` で設定が違うと、飛翔経路と落下分散は 1 つの KML にそろわない（:ref:`sec-kml`）。同じ出力先で重ねたいときは、``[sim]`` を含む設定を同じにして、``sim`` と ``dispersion`` の両方を実行し直す。
図形は地面に固定して描かれ、着地点は地形の起伏を考えない射点標高の面で決まる（:ref:`page-theory-limitations`）。

確認
------

結果が使えるかを、次の順に確かめる。

1. **終了ステータス。** 0 なら、すべてのケースまたはサンプルが完了し、安定で、係数表の範囲内である。3 のときは、出力は書かれているが結果は完全でないか、使えない（:ref:`sec-exit-status`）。標準エラー出力の ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行に理由とコードが出る。
2. **警告。** 警告は ``-q`` でも表示され、風の格子では ``dispersion_meta.json``、モンテカルロでは ``dispersion_summary.json`` の ``warnings`` に残る。

.. _sec-wind-grid-failed:

着地しなかったケース（風の格子）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``sim.max_time`` までに着地しないケースは失敗として数える。``dispersion.csv`` にも ``landed = false`` の行として残り、着地に関する列は ``NaN`` になる。
このケースは、着地距離の最大値と KML の輪郭・着地点から除かれる。したがって、失敗があるときの最大着地距離は **下限** にすぎない。

* 端末に、降下モードごとに ``max landing distance ≥ … m (N of M cases did not land: lower bound)`` と表示される。どのケースも着地しなければ ``NO CASE LANDED`` と表示され、距離は出ない。
* 警告に失敗の数が出て、``dispersion_meta.json`` の ``failed`` に数が入り、終了ステータス 3 で終わる（:ref:`IY-O028`、:ref:`IY-O530`）。

積分がエラーで止まったケース（数値の発散など）は、計算全体を止めずに失敗として記録される。``dispersion.csv`` のそのケースの行は ``landed = false`` で、列 ``error`` にエラーの文が入る。
``dispersion_meta.json`` の ``errored`` に数が入る（``failed`` はこれを含む）。警告には最初の 3 ケースのエラーが風速・風向・降下モードとともに出る（:ref:`IY-O531`、:ref:`IY-O532`、:ref:`IY-O533`）。
すべてのケースがエラーで止まったときは、出力を書いたうえで終了ステータス 1 で終わる（:ref:`IY-O011`）。

.. _sec-mc-failure:

失敗したサンプル（モンテカルロ）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ばらつきによっては飛翔が正常に終わらない。推力が小さすぎてランチャを離れない、傾けたレールの射角が範囲 :math:`(0, 90]` deg の外になる、``sim.max_time`` までに着地しない、などである。

* 失敗したサンプルは ``dispersion_mc.csv`` に ``status = failed`` の行として残る（結果の列は空欄）。**すべての統計から除かれ**、``failed`` として数えられる。着地しなかったサンプルの数は、``failed`` の内数として ``not_landed`` に別に数えられる。降下モードごとに判定され、実行の最後に理由が最大 3 件表示される。
* 失敗があると、その降下モードの統計は着地したサンプルだけを表し、真の広がりは報告値より大きいか不明である。``complete`` が ``false``、``max_landing_distance_is_lower_bound`` が ``true`` になり、端末には ``STATISTICS INCOMPLETE`` と最大着地距離の下限（``≥``）が表示される。終了ステータスは 3 である（:ref:`IY-O029`、:ref:`IY-O537`）。
* 着地したサンプルが 1 つだけの降下モードは、平均着地点だけを KML に入れ、誤差楕円は入れない。2 つ未満では、端末の楕円の行が ``n/a (needs at least 2 landed samples)`` になる。30 未満では ``only … landed samples: the statistics are rough`` の警告が出る（:ref:`IY-O538`）。
* どのサンプルも着地しなければ、統計と距離は ``null`` になり、``no sample landed`` と表示される。

.. _sec-dispersion-unstable:

静的に不安定なケース
~~~~~~~~~~~~~~~~~~~~~~~~

推力・質量・重心・空力係数などが公称値から外れるので、公称の飛翔が安定でも、一部のケースやサンプルが静的に不安定になりうる。静安定余裕の最小値が 0 cal 未満の飛翔を、静的に不安定とする。

* 不安定なケースは、降下モードごとに数えて警告するが、統計と最大着地距離には **入れたまま** にする。不安定な機体は転倒して飛び、着地点が大きく変わりうる。除くと分布の裾を過小に見積もるためである（:ref:`IY-O534`）。
* 終了ステータス 3 で終わり、標準エラー出力に ``RESULTS NOT USABLE: [IY-O025] N of M … are statically UNSTABLE (min static margin down to X cal): …`` の形の行が出る（:ref:`IY-O025`）。
* ``dispersion_meta.json``\ （風の格子）と ``dispersion_summary.json``\ （モンテカルロ）の ``unstable``\ （降下モードごとの数）と ``unstable_min_static_margin_cal`` に残る。``dispersion.csv`` と ``dispersion_mc.csv`` の最後から 2 番目の列 ``stable`` が ``false`` の行を取り出すと、不安定になった条件が分かる。
* 静安定余裕が求まらないケースは、判定できないものとして別に数えられ、警告が出て、終了ステータス 3 になる（``stability_unknown_runs``。:ref:`IY-O535`、:ref:`IY-O021`）。

対処: 静安定余裕を増やす（フィンを大きくする、重心を前へ寄せる）か、重心や空力係数の標準偏差を見直す。終了ステータスが 3 の結果は保安の判断に使わない。

.. _sec-dispersion-broadside:

横向きに落ちたケース
~~~~~~~~~~~~~~~~~~~~~~~~

弾道落下で、頂点のあと全迎角 60° から 120° の間（流れに対して横向き）で 10 s を超えて飛んだケースやサンプルを、横向きに落ちたものとして数える。
この着地点は、検証していない大迎角の空力係数で決まる（:ref:`page-theory-limitations`）。迎角 90° 付近の圧力中心が重心より前にある係数表で起こる。
サンプルでは、部品積み上げ法とパネル法のどちらの表でも大きな迎角の圧力中心が燃焼後の重心より後ろにあり、横向きに落ちた弾道落下はない。この検出は、大きな迎角の係数が粗い表や外部の表（``method = "table"``）、重心が後ろにある機体に備える確認である。

* 警告に ``warning[IY-O536]: 1 of 56 ballistic wind-grid cases fell broadside (angle of attack 60 to 120 deg for more than 10 s): …`` の形の文が出る（:ref:`IY-O536`）。モンテカルロ解析では ``wind-grid cases`` が ``Monte Carlo samples`` になる。
* ``broadside_runs`` に弾道落下の数が入り、``dispersion.csv`` と ``dispersion_mc.csv`` の最後の列 ``broadside_time`` に横向きで飛んだ時間 [s] が入る。
* 図では、その着地点に紫のひし形が重なり、見出し ``CAUTION`` の帯が付く（:ref:`sec-plots`）。

結果は使えないものとはせず、統計と最大着地距離に入れたままにする。終了ステータスは変わらない。
横向きに落ちたケースが最大着地距離を決めているときも、その距離を除かない。大迎角の係数の不確かさは大きいので、空力の手法を替えて計算し（:ref:`page-howto-keepout-zone` の手順 6）、大きいほうを使う。

うまくいかないとき
--------------------

.. list-table::
   :header-rows: 1
   :widths: 42 58

   * - 症状
     - 対処
   * - 終了ステータス 3 と ``INCOMPLETE RESULTS: [IY-O028]`` または ``[IY-O029]``
     - 着地しなかった、または失敗したケースがある。最大着地距離は下限である。``sim.max_time`` を延ばすか、失敗の理由を直す（:ref:`IY-O028`、:ref:`IY-O029`）
   * - 終了ステータス 3 と ``RESULTS NOT USABLE: [IY-O025]``
     - 不安定なケースがある。静安定余裕を増やすか、標準偏差を見直す（:ref:`IY-O025`）
   * - ``RESULTS NOT USABLE: [IY-O020]``、``[IY-O021]``
     - 公称の飛翔が不安定か、安定性を判定できない。機体の設定を直す（:ref:`IY-O020`、:ref:`IY-O021`）
   * - ``RESULTS NOT USABLE: [IY-O022]``、``[IY-O023]``、``[IY-O024]``
     - 飛翔が係数表の Mach 数または迎角の範囲を外れた。表の範囲を広げて作り直す（:ref:`IY-O022`、:ref:`IY-O023`、:ref:`IY-O024`）
   * - ``warning[IY-O512]``
     - 頂点が風の表の最上端より上である。最も高い頂点より上まで続く表で計算し直す（:ref:`IY-O512`）
   * - ``warning[IY-O514]``
     - 頂点が 1000 m を超えるので、``power`` または ``log`` の風は外挿値である。保安の検討には高度別の表を使う（:ref:`IY-O514`）
   * - ``error[IY-O011]``
     - すべてのケースがエラーで止まった。``dispersion.csv`` の ``error`` 列を読む（:ref:`IY-O011`）
   * - ``warning[IY-O518]``
     - 開傘の速さが 50 m/s または 2 kPa を超える。着地点はパラシュートが壊れないものとして計算しているので、この開傘は弾道落下に近い降下になりうるものとして扱う（:ref:`IY-O518`）

関連
------

* 保安距離（保安域の半径）を求める: :ref:`page-howto-keepout-zone`
* 落下分散の選び方、ばらつきの与え方、統計量: :ref:`page-theory-dispersion`
* 設定キー: :ref:`page-reference-config-dispersion`
* 出力ファイルの列と項目: :ref:`page-reference-output-files`
* 終了ステータス: :ref:`page-reference-exit-codes`
* 条件を変えて繰り返す: :ref:`page-howto-sweeps`
