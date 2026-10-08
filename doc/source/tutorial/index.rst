.. _page-tutorial:

はじめに
==========

このページでは、IgnisYeet をインストールし、付属のサンプル機体の飛翔を 1 回計算して、結果を見るところまでを順に行う。
所要時間は 10 分ほどである。各手順のコマンドはそのまま貼り付けて実行でき、実行後に出る内容を「期待する出力」として示す。
ここで使うサンプルは、直径 100 mm、全長 1.5 m、4 枚フィンの機体で、風速 4 m/s の北風、パラシュート降下の条件である。

前提
------

* Linux（x86_64、aarch64）または macOS（Intel、Apple Silicon）
* ``bash``、``curl``、``tar``\ （ほとんどの環境に入っている）
* インターネット接続

管理者権限は要らない。すべて自分のホームディレクトリの下（既定は ``~/.local``）に入る。

インストーラを実行する
------------------------

次の 1 行を端末に貼り付けて実行する。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash

質問が出るので、**Enter だけ** で答える（役割は user、CFD ツールは入れない、実行計画はそのまま進める）。
ただし、``~/.local/bin`` が ``PATH`` に入っていないときに出る「Add … to PATH in …?」の質問には ``y`` と答える。

期待する出力
   最後に ``IgnisYeet is ready`` の枠が出て、バージョン、実行ファイルとデータの場所、次に実行するコマンドが並ぶ（:numref:`fig-screen-install-done`）。
   画面の例では ``PATH`` の質問に ``n`` と答えたため、枠の下に ``PATH`` に加える行が出ている。

.. _fig-screen-install-done:

.. figure:: /_generated/screens/screen_install_done.png
   :width: 100%
   :alt: インストール完了の枠

   インストールの完了画面。枠の ``version`` がインストールした版、``binary`` が実行ファイルの場所である。``Next`` の下に、次の手順で打つコマンドが並ぶ。

新しい端末を開く（または ``source ~/.bashrc`` を実行する）と、``ignisyeet`` コマンドが使えるようになる。
質問や選択肢の詳細は :ref:`page-howto-install` にある。

動作を確かめる
----------------

バージョンを表示する。

.. code-block:: sh

   ignisyeet --version

期待する出力
   バージョンとライセンスの 3 行が出る（バージョンはインストールした版である）。

   .. code-block:: text

      IgnisYeet 0.4.0 — Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke).
      License GPL-3.0-or-later: <https://www.gnu.org/licenses/>.
      This is free software; there is NO WARRANTY.

``ignisyeet: command not found`` と出たときは、新しい端末を開いていないか、``PATH`` に ``~/.local/bin`` が入っていない。
``export PATH="$HOME/.local/bin:$PATH"`` を実行する。

.. _sec-run-first:

サンプルを実行する
--------------------

インストールされた例題を作業用の場所にコピーし、サンプル機体の飛翔を計算する。

.. code-block:: sh

   cp -r ~/.local/share/ignisyeet/current/examples ignisyeet-examples
   cd ignisyeet-examples
   ignisyeet sim sample.toml

空力係数表の作成から飛翔の計算まで、数秒で終わる。

期待する出力
   端末には枠付きの結果パネルが出る（:numref:`fig-screen-install-check-sim`）。
   主な値は次のとおりである。

   * 頂点高度 4440.5 m（飛翔時間 26.10 s）
   * 最大速度 518.8 m/s（Mach 1.54）
   * ランチャ離脱 0.25 s、45.7 m/s、静安定余裕 1.99 cal
   * 着地点 東 −655.6 m、北 −7707.7 m（685.0 s 後、着地速度 7.1 m/s）

   表示の最後に、書き出したファイルの一覧（``out/`` の下）と所要時間が出る。所要時間は計算機で変わる。

.. _fig-screen-install-check-sim:

.. figure:: /_generated/screens/screen_install_check_sim.png
   :width: 100%
   :alt: sim sample.toml の出力

   ``ignisyeet sim sample.toml`` の出力。上から、計算の流れ、``Flight result`` パネル、警告、書き出したファイル、所要時間である。

結果パネルの下に、黄色で ``[IY-O514] apogee 4440 m is above 1000 m, …`` という警告が 1 行出る。
これはエラーではない。サンプルの風のモデル（べき法則）を高度 1 km 以上へ外挿していることを知らせる注意で、計算は正常に終わっている。
警告の読み方は :ref:`page-howto-warnings-errors` にある。

結果を見る
------------

計算で ``out/`` に結果のファイルができる。主なものを確かめる。

.. code-block:: sh

   ls out

期待する出力
   .. code-block:: text

      aero_drag.csv   geometry.json   profile.csv   trajectory.csv
      aero_table.csv  ignisyeet.kml   summary.json
      aero_table.json

   並び方は端末の幅で変わる。

要約の値は ``summary.json`` にある。頂点高度、着地距離、静的に安定かどうかを取り出す。

.. code-block:: sh

   grep -E '"(apogee|landing_distance|stable)"' out/summary.json

期待する出力
   .. code-block:: text

      "apogee": 4440.476363486153,
      "landing_distance": 7735.52568470689,
      "stable": true,

   小数の下位の桁は環境で変わることがある。

次に、図を描く。

.. code-block:: sh

   ignisyeet-plot all out

期待する出力
   描いた図のファイルが ``out/plots/`` に書かれる。``sim`` だけを実行したので、パネル法と落下分散の図は出ない。

   .. code-block:: text

      wrote out/plots/geometry.png
      wrote out/plots/aero.png
      skip panel: panel_cp.csv not found
      wrote out/plots/trajectory_3d.png
      wrote out/plots/trajectory.png
      skip dispersion: dispersion.csv / dispersion_mc.csv not found

``out/plots/trajectory.png`` を画像ビューアで開くと、:numref:`fig-tutorial-trajectory` の図が見える。

.. _fig-tutorial-trajectory:

.. figure:: /_generated/plots/trajectory.*
   :width: 100%
   :alt: サンプルの飛翔経路の時間履歴

   サンプル機体の飛翔の時間履歴。見るのは、高度が頂点（4440.5 m）で最大になりその後パラシュートで降下していること、圧力中心と重心の距離（CP–CG の欄）が正のままであることである。

``out/ignisyeet.kml`` を Google Earth（Web 版でもよい）で開くと、飛翔経路と着地点を地図の上で見られる。
図の種類と KML の見方は :ref:`page-howto-plots-kml` にある。

次に読むページ
----------------

* 自分の機体を計算する：:ref:`page-howto-own-rocket`
* 結果の読み方（端末の表示、``summary.json``）：:ref:`page-howto-read-results`
* 風だけを変えた計算を繰り返す：:ref:`page-howto-sweeps`
* 設定ファイルの全項目：:ref:`page-reference-config`
* 計算がうまくいかないとき：:ref:`page-howto-warnings-errors`
