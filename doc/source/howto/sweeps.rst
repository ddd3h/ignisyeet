.. _page-howto-sweeps:

設定を変えて計算を繰り返す
============================

設定ファイルを書き換えずに、コマンドラインで風や空力の手法だけを変えて計算を繰り返す。
パネル法や CFD の係数表を一度だけ作り、風だけを変えた多くの計算で使い回す方法も扱う。

目的
------

* 風速や風向を変えた計算を、複数の出力先に分けて繰り返す。
* 時間のかかる係数表を 1 回だけ作って、使い回す。

前提
------

* サンプルの計算が動く（:ref:`page-tutorial`）。
* ``--set`` の書式の詳しい規則は :ref:`page-reference-cli` にある。ここでは手順に必要な範囲だけを使う。

手順
------

1. ``--set KEY=VALUE`` で設定を 1 つ変える。``KEY`` は設定ファイルの項目を ``.`` でつないだもの、``VALUE`` は TOML の値である。
   何度でも指定できる。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.speed=7 --set wind.direction_deg=90

   期待する結果：``Flight`` の行が ``wind 7.0 m/s from 90 deg`` になる。設定ファイルは書き換わらない。

   * 文字列の値は、``wind.model=profile`` のように引用符なしで書ける。
   * 同じキーを 2 回指定すると、最後の値を使い、警告を出す（:ref:`IY-C500`）。
   * 値の型や名前を誤ると、``error[IY-C005]: invalid config … with the --set overrides`` で止まり、原因の行が ``--set wind.speed=abc: invalid type: string "abc", expected f64`` の形で出る。
   * ``KEY=VALUE`` の形でない引数は、コマンドラインの使い方の誤りとして終了ステータス 2 で止まる（:ref:`IY-C002`）。
   * ``--set`` は項目を消せない。省略した場合と同じにしたい項目（二段開傘の ``recovery.main_cd_s`` と ``main_altitude`` など）は、元の設定ファイルに書かず、必要な計算でだけ ``--set`` で加える。

2. ``--out DIR`` で出力先を分ける。出力先を指定しないと、すべての計算が同じ ``out/`` に書き、前の結果を上書きする。
   ``--out`` の相対パスは、設定ファイルのディレクトリではなく、**現在のディレクトリ** から解釈する。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.speed=7 --set wind.direction_deg=90 --out runs/wind_7ms

   期待する結果：``runs/wind_7ms/`` に ``trajectory.csv``、``summary.json``、``ignisyeet.kml`` などが書かれる。

3. 風速を変えた計算を繰り返す。次の例は、風速 1 から 7 m/s の東風（``direction_deg = 90``）で ``sim`` を 7 回実行し、着地距離を並べる。

   .. code-block:: bash

      for v in 1 2 3 4 5 6 7; do
        ignisyeet sim sample.toml -q --set wind.speed=$v --set wind.direction_deg=90 --out runs/w${v}ms > /dev/null
        echo "$v m/s: $(grep -h '"landing_distance"' runs/w${v}ms/summary.json)"
      done

   ``-q`` は標準出力にファイルのパスだけを出し、エラーと警告を標準エラー出力に出す。``> /dev/null`` でパスを捨てると、警告だけが画面に残る。
   サンプルでは、風速 4 m/s で頂点が 1 km を超えるため、毎回 :ref:`IY-O514` の警告が出る。

   期待する結果：次の 7 行が出る（部品積み上げ法、サンプルの機体と射点。小数の下位の桁は環境で変わることがある）。

   .. code-block:: text

      1 m/s:   "landing_distance": 2614.0317273920805,
      2 m/s:   "landing_distance": 4572.92207030218,
      3 m/s:   "landing_distance": 6535.273304303107,
      4 m/s:   "landing_distance": 8496.338756128222,
      5 m/s:   "landing_distance": 10451.438904325232,
      6 m/s:   "landing_distance": 12392.931460473937,
      7 m/s:   "landing_distance": 14315.553970590654,

   風の条件を網羅して最大の着地距離を調べるときは、``ignisyeet dispersion`` が風速 × 風向の格子を並列に計算する（:ref:`page-howto-dispersion-run`）。

4. 係数表を 1 回だけ作り、使い回す。パネル法や CFD の表は作成に時間がかかるので、``aero`` で表を一度作り、``--aero-table`` で以後の計算に渡す。

   .. code-block:: sh

      ignisyeet aero sample.toml --set aero.method=panel --out runs/aero_panel
      ignisyeet sim  sample.toml --set aero.method=panel --aero-table runs/aero_panel \
          --set wind.speed=7 --set wind.direction_deg=90 --out runs/sim_panel_7ms

   期待する結果：1 行目が係数表 ``runs/aero_panel/aero_table.csv`` を書く（サンプルのパネル法で 9 秒ほど。20 スレッドの CPU、既定のパネル数）。
   2 行目は表を作り直さずに、その表で飛翔を計算する。パネル法の表を使ったサンプルの着地距離は、風速 1、4、7 m/s の東風でそれぞれ 2477.4 m、8019.8 m、13480.1 m である。

   ``--aero-table`` に渡す表は、**同じ機体と空力の設定で作ったもの** でなければならない。
   手法が違うなど、設定が合わないと、次のエラーで止まる。

   .. code-block:: text

      error[IY-O004]: --aero-table: runs/aero_panel/aero_table.csv was built for another rocket or other aero settings (cache key … vs … of this configuration); build it for this configuration with `ignisyeet aero` or leave out --aero-table

   ``--aero-table`` は ``aero.method = "table"`` とも（:ref:`IY-O002`）、``aero --force`` とも（:ref:`IY-O003`）併用できない。

.. _sec-output-lock:

同じ出力先を同時に使えないとき
--------------------------------

1 つの出力先で同時に動かせる実行は 1 つである。同じ出力先に対して、別の実行が動いている間に ``geom``、``aero``、``sim``、``dispersion`` を実行すると、何も書かずに次のエラーで止まる（終了ステータス 1。パスと番号は例）。

.. code-block:: text

   error[IY-O031]: another ignisyeet run is using runs/x (lock file runs/x/.ignisyeet.lock, pid 12345); wait for it or use a different --out (if no other run is active, delete runs/x/.ignisyeet.lock)

* 同時に計算するときは、``--out`` で出力先を分ける。
* 前の実行が強制終了された場合など、動いている実行がないのにこのエラーが出るときは、メッセージの末尾にあるとおり、``.ignisyeet.lock`` を手で消す。

確認
------

* 出力先ごとに ``summary.json`` ができ、``landed`` が ``true`` である。
* ``--aero-table`` を使った計算で、``aero`` の工程が再実行されていない（表の作成の行が出ない）。
* 同じ出力先を同時に使っていない（:ref:`IY-O031` が出ない）。

うまくいかないとき
--------------------

``error[IY-O004]``
   ``--aero-table`` の表が、この設定の機体や空力の設定で作ったものと違う。表を作ったときと同じ ``--set`` で ``aero.*`` を指定するか、``--aero-table`` を外して表を作り直す。

``error[IY-C005]``、``error[IY-C002]``
   ``--set`` の書式の誤りである。キーの綴り、値の型、``KEY=VALUE`` の形を確かめる。

``error[IY-O031]``
   上の「同じ出力先を同時に使えないとき」を見る。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-reference-cli`：``--set``、``--out``、``--aero-table`` の完全な規則
* :ref:`page-howto-dispersion-run`：風速 × 風向の格子と、モンテカルロ解析の実行
* :ref:`page-howto-keepout-zone`：``--set`` と ``--aero-table`` を使った保安距離の手順
