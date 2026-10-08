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

期待する結果：

.. code-block:: text

   Dispersion: 112 cases in 997.30ms -> out/dispersion.csv, out/dispersion_meta.json
     Ballistic: max landing distance 1549.6 m
     Parachute: max landing distance 14315.6 m

出力例の時間は CPU により異なる（コストの目安は :ref:`page-theory-dispersion`）。

``out/dispersion.csv``、``out/dispersion_meta.json`` に加えて、``[output] kml = true`` なら ``out/ignisyeet.kml`` が書かれる。

.. _sec-wind-grid-result:

2. 風の格子の結果を読む
~~~~~~~~~~~~~~~~~~~~~~~~~~

:numref:`fig-dispersion` は、同じ風速で風向だけを変えた着地点を線で結んだものである。内側から外側へ、風速が大きくなる。

.. _fig-dispersion:

.. figure:: /_generated/plots/dispersion.*
   :width: 100%

   サンプル機体の落下分散（射角 85°、射方位 270°、7 風速 × 8 方位）。左が弾道落下、右がパラシュート降下。多角形の中心の位置と、風速に応じた広がりの向きを見る。

見る点は次のとおりである。
着地点が風上側（弾道落下）と風下側（パラシュート降下）のどちらにずれるか、風速とともにどれだけ広がるかは :ref:`page-theory-descent` にある。

* **最大着地距離は、計算した条件の中の最大値である。** 風速と方位の刻みを変えると値が変わりうるので、刻みを細かくして確かめる（:ref:`sec-dispersion-grid-fineness`）。
* **多角形は近似である。** 方位が粗いと着地点が多角形（と KML の輪郭）の外に出る。保安域の判断には、多角形ではなく最大着地距離を半径とする円か、方位を 36 以上にした外側の包絡を使う（:ref:`sec-dispersion-grid-fineness`）。
* **弾道落下にはいびつな点が出ることがある。** 計算の誤りではない。安全の評価では、このような点を含めた最も外側の範囲で考える。

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

期待する結果（``seed = 1``、1000 サンプル、20 スレッドのマシンで 1.1 s）：

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

見る点は次のとおりである。

* **平均と楕円。** 弾道落下の平均着地点は射点から西に 1080 m、北に 370 m にあり、パラシュート降下の広がり（:math:`1\sigma`）は約 2 km で弾道落下より 1 桁大きい。楕円の定義は :ref:`fig-error-ellipse` にある。
* **楕円の内側の割合。** 正規分布なら :math:`1\sigma` 楕円に約 39 %、:math:`3\sigma` 楕円に約 99 % が入る。この計算では弾道落下で 44 % と 98 %、パラシュート降下で 41 % と 99 % で、分布は正規分布から少しずれている。
* **裾はパーセンタイルと最大値で見る。** 保安域の半径を考えるときは、楕円ではなく P90、P99、P99.9 と最大着地距離を使う（:ref:`sec-mc-percentile`）。P99.9 は着地したサンプルが 1000 以上のときだけ出る。最大値はサンプル数とともに大きくなる（:ref:`tbl-safety-samples`）。
* **頂点高度の平均。** 4480 m は公称の飛翔（4440 m）より 40 m ほど高い。頂点高度は推力や抗力係数に対して線形ではないので、平均は公称値と一致しない。

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
.. _sec-mc-failure:
.. _sec-dispersion-unstable:
.. _sec-dispersion-broadside:

終了ステータス 3 のとき
~~~~~~~~~~~~~~~~~~~~~~~~

結果が不完全か使えないと、出力は書かれたうえで終了ステータス 3 になり、標準エラー出力に ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行が出る。
次の表は、症状と、結果の扱いと、見るコードである。

.. list-table::
   :header-rows: 1
   :widths: 24 38 38

   * - 症状
     - 結果の扱い
     - コードと対処
   * - 着地しないケースやサンプルがある（``sim.max_time`` までに着地しない、推力が小さすぎてランチャを離れない、レールの射角が範囲外など）
     - そのケースは最大着地距離、統計、KML の輪郭から除かれる。最大着地距離は下限（``≥``）で、``complete`` が ``false`` になる。全部が着地しなければ距離は出ない
     - :ref:`IY-O028`、:ref:`IY-O029`、:ref:`IY-O530`、:ref:`IY-O537`。``sim.max_time`` を延ばすか、失敗の理由を直す。``dispersion.csv`` の ``error`` 列と ``dispersion_mc.csv`` の ``status`` 列を読む
   * - 積分がエラーで止まったケースがある
     - 失敗として数える。すべてのケースが止まると終了ステータス 1 になる
     - :ref:`IY-O531`、:ref:`IY-O532`、:ref:`IY-O533`、:ref:`IY-O011`
   * - 公称の飛翔が不安定か、安定性を判定できない
     - 結果は使えない
     - :ref:`IY-O020`、:ref:`IY-O021`。機体の設定を直す
   * - 飛翔が係数表の Mach 数または迎角の範囲を外れた
     - 結果は使えない
     - :ref:`IY-O022`、:ref:`IY-O023`、:ref:`IY-O024`。表の範囲を広げて作り直す
   * - 着地したサンプルが少ない
     - 1 つだけなら平均着地点だけ、2 未満なら楕円は出ない。30 未満の統計は粗い
     - :ref:`IY-O538`。サンプル数を増やす
   * - 静的に不安定なケースがある
     - 統計と最大着地距離に入れたままにする。不安定な機体は転倒して飛び、除くと裾を過小に見積もるためである。``stable`` が ``false`` の行が該当する条件である
     - :ref:`IY-O025`、:ref:`IY-O534`、:ref:`IY-O535`。静安定余裕を増やすか、重心や空力係数の標準偏差を見直す
   * - 横向きに落ちた弾道落下がある（迎角 60° から 120° で 10 s を超えて飛ぶ。``broadside_runs`` と ``broadside_time``）
     - 結果は使えないものとはせず、統計に入れたままにする。終了ステータスは変わらない。この着地点は検証していない大迎角の空力係数で決まる（:ref:`page-theory-limitations`）
     - :ref:`IY-O536`。距離を除かず、空力の手法を替えて計算し（:ref:`page-howto-keepout-zone` の手順 6）、大きいほうを使う

終了ステータス 3 の結果は保安の判断に使わない（:ref:`sec-exit-status`）。

うまくいかないとき
--------------------

終了ステータス 3 とエラーの症状は前の表にある。警告は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 42 58

   * - 症状
     - 対処
   * - ``warning[IY-O512]``
     - 頂点が風の表の最上端より上である。最も高い頂点より上まで続く表で計算し直す（:ref:`IY-O512`）
   * - ``warning[IY-O514]``
     - 頂点が 1000 m を超えるので、``power`` または ``log`` の風は外挿値である。保安の検討には高度別の表を使う（:ref:`IY-O514`）
   * - ``warning[IY-O518]``
     - 開傘の速さが 50 m/s または 2 kPa を超える。着地点はパラシュートが壊れないものとして計算しているので、この開傘は弾道落下に近い降下になりうるものとして扱う（:ref:`IY-O518`）

関連
------

* 保安距離（保安域の半径）を求める：:ref:`page-howto-keepout-zone`
* 落下分散の選び方、ばらつきの与え方、統計量：:ref:`page-theory-dispersion`
* 設定キー：:ref:`page-reference-config-dispersion`
* 出力ファイルの列と項目：:ref:`page-reference-output-files`
* 終了ステータス：:ref:`page-reference-exit-codes`
* 条件を変えて繰り返す：:ref:`page-howto-sweeps`
