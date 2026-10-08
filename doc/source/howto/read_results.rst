.. _page-howto-read-results:

結果を読む
============

``sim`` や ``dispersion`` が終わったあとに、端末の表示、終了ステータス、``summary.json`` を見て、結果が使えるかどうかを判断する。

目的
------

* 結果パネルのどの行を見るかを知る。
* 結果が使える、不完全、使えない、のどれかを見分ける。

前提
------

* ``ignisyeet sim`` を 1 回実行している（:ref:`page-tutorial`）。

手順
------

1. 起動時の設定パネルで、入力の取り違えがないか確かめる。
   ``ignisyeet`` は端末で実行すると、バナーに続いて設定パネル（Configuration）を表示する。入力ファイル、機体、射点、回収、空力、環境（地球・大気・風）、積分、出力先が項目ごとに並ぶ。
   単位や風向の入れ間違いに、実行前に気づける（:numref:`fig-screen-sim-start`）。

   期待する結果：設定ファイルに書いた値が、項目ごとに表示される。

   .. _fig-screen-sim-start:

   .. figure:: /_generated/screens/screen_sim_start.png
      :width: 100%
      :alt: sim の起動画面（バナーと設定パネル）

      ``sim`` の起動画面。水色の見出しで項目が分かれ、設定値は太字で示す。見るのは、機体の質量、射点、風が意図した値になっていることである。

2. 計算の進み具合を見る。工程ごとに、実行中は進捗バーを、終わると完了行（チェック印と処理名、要約、所要時間）を表示する。

   ``aero``
      Barrowman 法では係数表の行数を分母にしたバーを出す。パネル法では段階ごとに進み具合を示し、2 行目に実行中の処理を表示する。保存済みの表を使うときは、バーを出さずに ``cached`` と行数を表示する（:numref:`fig-screen-aero-panel`）。

   ``sim``
      飛翔の段階（``rail`` ``powered`` ``coast`` ``parachute`` など）と、その時点の高度・速度・Mach 数・静安定余裕を表示し続ける。

   ``dispersion``
      終わったケース数のバーに、1 秒あたりの処理数、経過時間、残り時間の見積り（ETA）を表示する。2 行目に、ここまでの最大着地距離（弾道落下とパラシュート降下）が出る。
      分母は、風の格子では風速 × 風向 × 降下モードのケース数、モンテカルロ解析ではサンプル数 × 降下モードの数（パラシュートが有効なら 2）である（:numref:`fig-screen-dispersion`）。

   期待する結果：すべての工程に緑のチェックが付いて終わる。

   .. _fig-screen-aero-panel:

   .. figure:: /_generated/screens/screen_aero_panel.png
      :width: 100%
      :alt: aero のパネル法の実行中の画面

      ``aero``\ （``method = "panel"``）の実行中。形状の要約に続いて、段階の進み具合（4/9）と ETA、実行中の処理が出る。

   .. _fig-screen-dispersion:

   .. figure:: /_generated/screens/screen_dispersion.png
      :width: 100%
      :alt: dispersion（モンテカルロ）の実行中の画面

      ``dispersion``\ （``mode = "monte_carlo"``）の実行中。処理数、1 秒あたりの処理数、ETA、ここまでの最大着地距離が出る。処理数は、弾道落下とパラシュート降下を別々に数える。

3. 結果パネルを読む。完了すると、``sim`` は ``Flight result`` パネルを出す（:numref:`fig-screen-sim-result`）。見る行は次のとおりである。

   .. list-table::
      :header-rows: 1
      :widths: 22 78

      * - 行
        - 見ること
      * - ``wind``
        - 計算に使った風（風速と、風が吹いてくる方位）。``profile`` では表の代表風になる。
      * - ``rail exit``
        - ランチャ離脱の時刻、速度、静安定余裕。離脱速度が 15 m/s を下回ると警告が出る。
      * - ``burnout``
        - 燃焼終了の時刻。飛翔が推力曲線の終わりより前に終わったときは ``not reached`` と出る。
      * - ``max speed``
        - 最大速度（Mach 数）と最大動圧。
      * - ``min margin``
        - 静安定余裕の最小値と、その時刻、Mach 数。判定の印が付く（下の表）。
      * - ``apogee``
        - 頂点の高度（射点からの高さ）と時刻。
      * - ``landing``
        - 着地点（射点からの東・北の距離、緯度・経度）、着地までの時間、着地速度。着地速度は地面に対する速さで、横方向の速さを含む。
      * - ``main deploy``
        - 二段開傘のとき、メインが開いた高度と時刻。

   期待する結果：サンプルでは、頂点 4440.5 m、静安定余裕の最小値 1.99 cal（ランチャ離脱時）、着地点が東 −655.6 m、北 −7707.7 m、着地速度 7.1 m/s と出る。

   .. _fig-screen-sim-result:

   .. figure:: /_generated/screens/screen_sim_result.png
      :width: 100%
      :alt: sim の完了画面（結果パネルと出力ファイル）

      ``sim`` の完了画面。上から、工程の完了行、``Flight result`` パネル、警告、書き出したファイルの一覧、所要時間である。見るのは ``min margin`` の印と、パネルの下の警告の有無である。

   静安定余裕には、次の判定の印が付く。

   .. list-table::
      :header-rows: 1
      :widths: 28 32 40

      * - 静安定余裕
        - 表示
        - 意味
      * - 1.5 cal 以上
        - 緑のチェック印
        - 十分な余裕がある
      * - 1.0 cal 以上 1.5 cal 未満
        - 黄の警告記号と ``marginal (< 1.5 cal)``
        - 余裕が小さい。突風や重心のずれで不安定に近づく
      * - 0 cal 以上 1.0 cal 未満
        - 赤の記号と ``low static margin``
        - 余裕が不足している。設計を見直す
      * - 0 cal 未満
        - 赤のバツ印と ``UNSTABLE``
        - 不安定（圧力中心が重心より前にある）。結果は使えない

   ここでの余裕は、迎角 0 の極限の小迎角の余裕で、その状態が飛んでいる迎角は入らない。自由飛行中の最小値は、ランチャ離脱から頂点までのうち対気速度が 10 m/s を超える間の値である。
   サンプルでは燃焼中に重心が前へ移るので、最小はランチャ離脱の時刻にある。
   頂点まで対気速度が 10 m/s を超えなかったときは、自由飛行中の値が ``n/a (airspeed < 10 m/s)`` と出る。
   余裕の定義と求め方は :ref:`page-theory-dynamics` にある。

4. 終了ステータスを確かめる。

   .. code-block:: sh

      ignisyeet sim rocket.toml
      echo $?

   期待する結果：``0`` なら、計算は成功している。``3`` のときは、出力は書いたが結果が不完全か使えない（手順 5）。``1`` はエラー、``2`` はコマンドラインの使い方の誤りである（:ref:`page-howto-warnings-errors`）。

5. 結果が使えるかを見分ける。終了ステータスが 3 のとき、標準エラー出力の最後の行が理由を示す。

   .. list-table::
      :header-rows: 1
      :widths: 24 40 36

      * - 行の頭
        - 意味
        - 扱い
      * - ``INCOMPLETE RESULTS``
        - 結果は書いたが不完全。``sim.max_time`` までに着地しなかった、または失敗や着地しなかったケース、サンプルがある。
        - 理由を直して再実行する。着地しなかった飛翔の着地点は存在しない。
      * - ``RESULTS NOT USABLE``
        - 静的に不安定、静的な安定を判定できない、係数表の迎角または Mach 数の範囲を超えて係数を引いた、のいずれか。
        - 結果を判断に使わない。特に保安の判断に使わない。

   不安定な機体の例として、サンプルの ``rocket.cg_dry`` を 1.3 m にすると、次のように出る。

   .. code-block:: text

      warning[IY-O506]: static margin -1.49 cal at t = 5.96 s, Mach 0.03 is negative: the centre of pressure is forward of the CG, the rocket is UNSTABLE
      ignisyeet: RESULTS NOT USABLE: [IY-O020] the rocket is statically UNSTABLE (min static margin -1.49 cal at t = 5.96 s, Mach 0.03) (exit status 3)

   ``sim.max_time`` を 100 s にすると、サンプルは着地せず、次のように出る。

   .. code-block:: text

      landing         NONE: the rocket did not reach the ground within sim.max_time = 100 s
      warning[IY-O504]: THE FLIGHT DID NOT LAND within sim.max_time = 100 s: there is no landing point; raise sim.max_time or check the descent (parachute, drag area, launch angle)
      ignisyeet: INCOMPLETE RESULTS: [IY-O027] the flight did not land within sim.max_time = 100 s (exit status 3)

   行頭の ``IY-…`` から、各コードのページに進める（:ref:`page-howto-warnings-errors`）。

6. ``summary.json`` で、機械的に判定できる項目を確かめる。スクリプトから結果を使うときは、次の項目を見る。

   .. list-table::
      :header-rows: 1
      :widths: 28 72

      * - 項目
        - 見ること
      * - ``stable``
        - ``true`` なら静的に安定、``false`` なら不安定、``null`` なら判定できない。``true`` 以外は結果を使わない。
      * - ``landed``
        - ``false`` なら着地しなかった。着地に関する項目は ``null`` になる。
      * - ``aero_table_range``
        - ``angles_in_range`` と ``machs_in_range`` が ``true`` であること。``false`` なら、係数表の範囲外の係数を使った結果で、使えない。
      * - ``warnings``
        - 警告の配列。各文字列は ``[IY-O514] …`` のようにコードを角括弧で先頭に付ける。空でなければ、すべて読む。

   サンプルでは、``stable`` が ``true``、``landed`` が ``true``、``angles_in_range`` と ``machs_in_range`` が ``true`` で、``warnings`` に ``[IY-O514] …`` の 1 件がある。
   全項目は :ref:`sec-file-summary` にある。

   期待する結果：``stable`` が ``true``、``landed`` が ``true``、``aero_table_range`` の 2 項目が ``true``。

確認
------

結果を使う前に、次をすべて確かめる。

* 終了ステータスが 0 である。
* ``stable`` が ``true`` で、``landed`` が ``true`` である。
* ``warnings`` の各行の意味を理解している。
* ``dispersion`` の結果は、手順 5 の行に加えて、不安定なケースと着地しなかったケースの数を見る（:ref:`page-howto-dispersion-run`）。

うまくいかないとき
--------------------

``RESULTS NOT USABLE`` が出る
   静的に不安定なら、重心を前へ移すか、フィンを大きくして、静安定余裕を増やす。係数表の範囲外なら、``aero.alpha_max_deg`` や ``aero.mach_max`` を大きくして表を作り直す。コードごとの対処は :ref:`errors-cli` にある。

``INCOMPLETE RESULTS`` が出る
   着地しなかった原因を確かめる。降下方式（パラシュートの有無と抗力面積）、``sim.max_time``、風の設定を見直す。

端末の表示が崩れる、色が出ない
   標準出力が端末でないとき（パイプやファイルへの出力）は、装飾のない行単位のプレーンモードになる。``-q``、``--no-progress``、``NO_COLOR`` などの出力モードは :ref:`sec-ui-modes` にある。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-howto-warnings-errors`：エラーとワーニングの読み方、終了ステータス
* :ref:`page-howto-plots-kml`：図と KML で結果を見る
* :ref:`page-howto-dispersion-run`：落下分散の結果の読み方
* :ref:`sec-file-summary`：``summary.json`` の全項目
* :ref:`sec-ui-modes`：出力モードと環境変数
