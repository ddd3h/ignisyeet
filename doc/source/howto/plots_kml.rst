.. _page-howto-plots-kml:

図と KML で結果を見る
=======================

``python/plot.py``（インストーラで入れたときは ``ignisyeet-plot``）で計算結果の図を描き、``ignisyeet.kml`` を Google Earth で開いて飛翔経路と着地点を地図の上で確かめる。

目的
------

* 形状、空力係数、飛翔経路、落下分散の図を描く。
* 結果に問題があるときの図の見出し（帯）と印を読み取る。
* 飛翔経路と落下分散を Google Earth で見る。

前提
------

* 計算が終わっている（図の元になる CSV と JSON が出力先にある）。
* 作図環境がある。インストーラで入れたときは ``ignisyeet-plot`` が使える。``pip`` で作る方法は :ref:`sec-python-env` にある。
  リポジトリから使うときは ``uv run --project python python/plot.py`` を ``ignisyeet-plot`` の代わりに使う。

手順
------

1. すべての図を描く。

   .. code-block:: sh

      ignisyeet-plot all out

   期待する結果：``out/plots/`` に、元のデータがある図の PNG ができる。サンプルを ``sim`` だけ実行したときは次のとおりである。

   .. code-block:: text

      wrote out/plots/geometry.png
      wrote out/plots/aero.png
      skip panel: panel_cp.csv not found
      wrote out/plots/trajectory_3d.png
      wrote out/plots/trajectory.png
      skip dispersion: dispersion.csv / dispersion_mc.csv not found

   ``all`` は、元のデータがない図を ``skip …`` と表示して飛ばす。

2. 描く図を選ぶ。``geometry``、``aero``、``panel``、``trajectory``、``dispersion`` のいずれか 1 つを指定する。

   .. code-block:: sh

      ignisyeet-plot trajectory out

   図と元のデータの対応は次のとおりである。

   .. list-table::
      :header-rows: 1
      :widths: 22 30 48

      * - サブコマンド
        - 元のデータ
        - 描く図
      * - ``geometry``
        - ``profile.csv``
        - ``geometry.png``：胴体半径とフィンの高さ（抽出結果の確認）
      * - ``aero``
        - ``aero_table.csv``、``aero_drag.csv``
        - ``aero.png``：迎角 0 の抗力係数 :math:`C_{D0}` の内訳、法線力係数の傾き :math:`C_{N\alpha}`、圧力中心 :math:`x_{cp}`、法線力係数と迎角
      * - ``panel``
        - ``panel_cp.csv``
        - ``panel.png``：パネル法の表面の圧力係数（``method = "panel"`` のとき）
      * - ``trajectory``
        - ``trajectory.csv``、``summary.json``
        - ``trajectory.png``：高度、対気速度、Mach 数、迎角、圧力中心と重心の距離、動圧、推力、質量、ピッチ角、横滑り角、ロール角速度の時間履歴。``trajectory_3d.png``：飛翔経路の 3 次元表示
      * - ``dispersion``
        - ``dispersion.csv``、``dispersion_meta.json``（風の格子）、``dispersion_mc.csv``、``dispersion_summary.json``（モンテカルロ）
        - ``dispersion.png``：風速ごとの着地点の輪郭。``dispersion_mc.png``：モンテカルロの着地点と誤差楕円

   期待する結果：選んだ図の PNG が ``out/plots/`` に書かれる。

3. 2 つの係数表を比べるときは、``aero`` に ``--compare`` を付ける。

   .. code-block:: sh

      ignisyeet-plot aero out_barrowman --compare out_panel

   期待する結果：``aero_compare.png`` に、2 つの表の :math:`C_{N\alpha}`、:math:`x_{cp}`、:math:`C_{D0}` が Mach 数に対して重なって描かれる。凡例はディレクトリ名である。

4. 図の見出しの帯を読む。結果が使えない、不完全、または注意が要るとき、飛翔経路の図と落下分散の図の上に、目立つ帯を描く。帯と印の意味は、次の「図の内容」にある。

   期待する結果：問題のない結果では帯が出ない。

.. _sec-plots:

図の内容
----------

帯の見出しは、``summary.json``\ （飛翔経路）、``dispersion_meta.json``\ （風の格子）、``dispersion_summary.json``\ （モンテカルロ）の項目から決まり、理由を 1 行ずつ並べる。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 見出し（帯の色）
     - 条件
   * - ``RESULTS NOT USABLE``\ （赤）
     - 静的に不安定（``stable`` が ``false``）、静的な安定を判定できない（``stable`` が ``null``）、係数表の迎角の範囲外の引き出しがある（``aero_table_range`` の ``angles_in_range`` が ``false``）、Mach 数の範囲外の引き出しがある（``machs_in_range`` が ``false``）。落下分散では、公称の飛翔のほか、不安定または判定できない飛翔を含む降下モードも数える
   * - ``INCOMPLETE``\ （橙）
     - 飛翔が着地しなかった（``landed`` が ``false``）、失敗または着地しなかったケースやサンプルがある
   * - ``CAUTION``\ （橙）
     - 弾道落下が横向きに落ちた（``fell_broadside`` が ``true``、または ``broadside_runs`` が 1 以上）。``large-angle aerodynamics, not validated`` と書く

条件が重なるときは、最も重い見出し（``RESULTS NOT USABLE``、``INCOMPLETE``、``CAUTION`` の順）を出し、理由はすべて並べる。
これらの項目を持たない JSON では帯を描かない。

落下分散の図では、次の着地点に印を重ねる。印の凡例には数を添える。

* 静的に不安定な飛翔：赤のバツ印（``statically unstable``）。
* 静的な安定を判定できない飛翔：橙の白抜きの三角（``stability unknown``）。
* 横向きに落ちた弾道落下：紫の白抜きのひし形（``fell broadside``）。

風の格子の図では、風速ごとの線は風速の大きい順に並ぶ凡例とともに描く。最も大きい風速の線が最も外側とは限らない（弾道落下では、途中の風速で最も遠くなることがある）。
``wind.model = "profile"`` の結果では、凡例の見出しが ``profile mean speed, highest first`` になる。
横向きに落ちたケースのところで、風向の順に結ぶ線を切る。
モンテカルロの図には、最大着地距離と、着地距離の P50 / P90 / P99 / P99.9 を、枠で囲んで軸の下に書き添える。着地したサンプルが 1000 未満のときは P99.9 を書かず、``(P99.9 needs ≥ 1000 landed samples)`` と添える。

飛翔経路の時間履歴の図では、高度の欄だけが着地までの全体を描く。ほかの欄は、パラシュートの開傘まで（弾道落下では着地まで）である。
推力の欄は燃焼終了の時刻の 1.2 倍まで、圧力中心と重心の距離の欄は、ランチャを離れてから頂点までの上昇中を描く。
この欄は、その時刻の迎角での圧力中心と重心の距離 [cal] で、静的な安定の判定に使う小迎角の余裕とは別の量である（:ref:`page-howto-read-results`）。
二段開傘の飛翔では、軌跡の図にメインの開傘の位置を四角の印で示す。
飛翔経路の図の高度は、``summary.json`` の頂点高度と同じ定義の、射点からの高さである。

.. _sec-kml:

Google Earth で見る（KML）
----------------------------

``[output]`` の ``kml`` が ``true``\ （既定）のとき、``sim`` と ``dispersion`` は ``ignisyeet.kml`` を、CSV や JSON と同じ出力先に書き出す。
KML は地理情報の XML 形式で、OGC の標準（KML 2.2 :cite:`ogckml`）である。
Google Earth（デスクトップ版・Web 版）や QGIS で開くと、機体の経路や着地点を衛星写真の上に重ねて確認できる。
不要なときは ``kml = false`` にする。各要素をクリックすると、時刻・速度・着地点などの数値が説明欄に出る（説明欄の文は英語である）。

1. ``ignisyeet.kml`` を開く。Google Earth の Web 版では、ファイルの読み込みから開く。デスクトップ版では、ファイルをダブルクリックする。

   期待する結果：左のパネルに、次のフォルダが並び、チェックボックスで表示を切り替えられる。該当する結果がなければ、そのフォルダは作られない。

   .. list-table::
      :header-rows: 1
      :widths: 28 72

      * - フォルダ
        - 内容
      * - ``Launch site``
        - 射点を示す白いピン。説明欄に射角、方位、レール長が載る。
      * - ``Flight``
        - ``sim`` の飛翔。経路を段階ごとに色分けする（黄がランチャ滑走、赤が燃焼中、青が燃焼後の慣性飛行、緑がパラシュート降下）。イベントの印は、ランチャ離脱（静安定余裕を含む）、燃焼終了、頂点、開傘、着地である。
      * - ``Dispersion – wind grid``
        - 風の格子の結果。降下モード（``ballistic`` と ``parachute``）ごとに、風速ごとの輪郭（同じ風速で風向を変えた着地点を風向の順に結んだ閉じた線）と、各ケースの着地点がある。弾道落下は青、パラシュート降下は橙で、風速が大きいほど濃い。
      * - ``Dispersion – Monte Carlo``
        - モンテカルロの結果。降下モードごとに、平均着地点、:math:`1\sigma` と :math:`3\sigma` の誤差楕円、各サンプルの着地点がある。

2. 飛翔経路と落下分散を同じ画面で重ねて見る。``sim`` と ``dispersion`` は、実行のたびに ``ignisyeet.kml`` を書き直し、同じ出力先にある他の結果も読み戻して 1 つにまとめる。
   したがって、``sim`` のあとに ``dispersion`` を実行すると、両方が入る。
   設定を変えたときは、``sim`` と ``dispersion`` の **両方** を実行し直す。現在の設定と一致しない前の結果は、古いものとして KML に入れず、警告を出す（:ref:`IY-O539`）。

   期待する結果：``Flight`` と ``Dispersion`` のフォルダが 1 つの KML にそろう。

   着地しなかった飛翔と、着地しなかった風のケースは KML に入れない。モンテカルロでは、着地したサンプルだけを入れる。誤差楕円は、着地したサンプルが 2 つ未満の降下モードでは入れない。
   ``sim`` がエラーで止まったときは、前の飛翔が残らないよう、飛翔の層を除いて書き直す。

3. 高度を確かめる。飛翔経路の高度は絶対高度（WGS84 楕円体からの高さ）で書き出す。射点で ``launch.altitude`` から始まり、飛翔中は ``launch.altitude`` に射点からの上向きの高さを足した値である。
   KML の絶対高度は名目上は平均海面（EGM96 ジオイド）からの高度だが、IgnisYeet はジオイド高（楕円体と平均海面の差）を **扱わない**。場所によっては数十 m の差になる。
   ``launch.altitude`` には射点の **海抜標高** を入れ、地形の高さと合っていることを確かめる。合っていないと、経路が地面にめり込んだり浮いたりして見える。
   射点・着地点・誤差楕円など地面に固定する図形は、Google Earth の地形に沿って表示される。

   期待する結果：射点のピンが地表に立ち、飛翔経路が地面から始まる。

.. _sec-run-troubleshooting:

うまくいかないとき
--------------------

作図が表示するメッセージと意味は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 46 54

   * - メッセージ
     - 意味と対処
   * - ``error: output directory … not found; run the ignisyeet commands first``
     - 出力先がない。先に ``ignisyeet`` で計算するか、パスを直す。終了ステータス 1。
   * - ``error: nothing to plot in …``
     - 描ける図が 1 つもない。計算を実行してから描く。終了ステータス 1。個別の図（``geometry`` など）を選んで何も描けなかったときも、``nothing to plot for …`` のエラーで終了ステータス 1 になる。
   * - ``skip geometry: profile.csv not found (run `ignisyeet geom` again)``
     - 形状の図の元がない。``ignisyeet geom`` を実行する。
   * - ``skip trajectory: the last sim run failed: …; not plotting the old trajectory``
     - 直前の ``sim`` が失敗している（``summary.json`` が失敗の印）。古い結果を描かない。失敗の原因を直して ``sim`` を実行し直す。``trajectory`` を直接指定したときは、``error: the last sim run failed: …`` で終了ステータス 1 になる。
   * - ``skip dispersion_mc: dispersion_summary.json not found (run `ignisyeet dispersion` again)``
     - モンテカルロの図の元の一部がない。``ignisyeet dispersion`` を実行し直す。
   * - ``skip panel: panel_cp.csv not found``
     - パネル法の結果がない。``aero.method = "panel"`` で計算したときだけ出る図なので、通常は無視してよい。
   * - ``removed … (plot skipped; the file was from an earlier run)``
     - 今回は描かなかった図の古い PNG が ``plots/`` に残っていたため、取り違えないよう消した。

``sim`` や ``dispersion`` が終了ステータス 3（出力は書いたが不完全、または使えない）で終わったときは、図の上に帯が付く。帯の意味は上の「図の内容」にある。
エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-howto-read-results`：端末の表示と ``summary.json`` の読み方
* :ref:`page-howto-dispersion-run`：落下分散の実行と結果の読み方
* :ref:`page-reference-output-files`：出力ファイルの列と項目
* :ref:`page-reference-cli`：作図スクリプトのオプション
