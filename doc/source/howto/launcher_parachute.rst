.. _page-howto-launcher-parachute:

射点とパラシュートを設定する
==============================

射点、射角、ランチャのレール長、パラシュート（単一または二段開傘）を設定し、結果で離脱速度と開傘の速さを確かめる。

目的
------

* 射点と射角を決めて、ランチャ離脱までの条件を確かめる。
* パラシュートの抗力面積と開傘の遅れ（二段開傘ではメインの開傘高度も）を設定する。
* 弾道落下とパラシュート降下のどちらで計算するかを選ぶ。

前提
------

* 機体とモータの設定ができている（:ref:`page-howto-own-rocket`、:ref:`page-howto-motor`）。
* 射場の緯度・経度・標高、使うランチャのレール長、パラシュートの抗力面積 :math:`C_DS`。

手順
------

1. 射点を書く。緯度・経度は WGS84 の度、標高は海抜 [m] である。

   .. code-block:: toml

      [launch]
      latitude = 34.7347
      longitude = 139.4204
      altitude = 20.0            # 射点の海抜標高 [m]

   標高は海抜の値を入れる。KML で地形に重ねるとき、この値が地形の高さと合っていないと経路が地面にめり込んだり浮いたりして見える（:ref:`sec-kml`）。

   期待する結果：``ignisyeet sim`` が設定を読み込み、着地点が緯度・経度で表示される。

2. 射角と射方位を書く。

   .. code-block:: toml

      elevation_deg = 85.0       # 水平面からの仰角 [deg]（0 より大きく 90 以下）
      azimuth_deg = 270.0        # 射方位。北から時計回り [deg]（270 は西向き）
      # roll_deg = 0.0           # ランチャ上の機体のロール角 [deg]。0 では y_b が水平

   射方位は、傾けた機体のノーズが向く水平方位であり、風向（風が吹いてくる方位）とは別の量である。
   各項目は :ref:`launch.elevation_deg <cfg-launch-elevation_deg>`、:ref:`launch.azimuth_deg <cfg-launch-azimuth_deg>`、:ref:`launch.roll_deg <cfg-launch-roll_deg>` にある。

   期待する結果：``sim`` が設定を読み込み、エラーなく計算する。範囲外の値（射角が 0 以下または 90 を超える、など）はエラーで止まる。

3. レール長を書く。

   .. code-block:: toml

      rail_length = 5.0          # 機体が発射位置からレールに沿って進む距離 [m]

   ``rail_length`` は、機体が発射位置からレールに沿って進み、レールを離れるまでの有効な案内長である。
   後端のラグがレールの端にないときは、レールの物理的な長さではなく、ラグが発射位置からレールの端まで動ける距離を入れる（:ref:`sec-rail`）。

   期待する結果：``sim`` の ``rail exit`` の行に、離脱の時刻と速度と、離脱時の静安定余裕が出る。サンプル（レール長 5 m）では ``0.25 s, 45.7 m/s, stability 1.99 cal`` である。

4. 離脱速度と風を確かめる。離脱速度は 15 m/s 以上、風による迎角は 15° 以下を目安にする。
   これを外れると、次の警告が出る。

   * ``rail exit speed … m/s is below 15 m/s``（:ref:`IY-O515`）：フィンの効きが小さく、機体がレールの線から風で流されやすい。レールを長くするか、推力重量比を上げる。
   * ``the wind angle of attack at rail exit is … deg``（:ref:`IY-O516`）：離脱の高さの風と離脱速度から求めた迎角が 15° を超える。機体が風上へ強く向きを変える。

   サンプルのレール長を変えたときの離脱速度は次のとおりである（部品積み上げ法、風 4 m/s、射角 85°）。

   .. list-table::
      :header-rows: 1
      :widths: 30 35 35

      * - ``launch.rail_length``
        - 離脱の時刻と速度
        - 警告
      * - 5.0 m（サンプル）
        - 0.25 s、45.7 m/s
        - なし
      * - 1.0 m
        - 0.12 s、20.0 m/s
        - なし
      * - 0.5 m
        - 0.10 s、13.9 m/s
        - :ref:`IY-O515`
      * - 0.3 m
        - 0.08 s、10.6 m/s
        - :ref:`IY-O515`

   風速 12 m/s、レール長 2 m にすると、離脱速度は 28.5 m/s で、風による迎角は 22.8° になり :ref:`IY-O516` が出る。

   期待する結果：想定する風の範囲で、警告が出ない。

5. 単一のパラシュートを設定する。

   .. code-block:: toml

      [recovery]
      enabled = true
      cd_s = 3.5                 # 抗力面積 C_D S [m^2]
      delay = 1.0                # 頂点から開傘までの遅れ [s]

   ``delay`` は頂点から開傘までの時間である。
   頂点 + ``delay`` にモータがまだ燃えているときは、開傘を燃焼終了まで遅らせて警告する（:ref:`IY-O517`）。
   ``enabled = false`` のときは、落下分散を含めて弾道落下だけを計算する。

   期待する結果：端末の設定パネルに終端降下速度の見積り（``terminal speed``）が出る。サンプル（乾燥質量 8 kg、:math:`C_DS = 3.5\ \mathrm{m^2}`）では 6.0 m/s である。
   これは海面の標準密度と乾燥質量から求めた見積りで、着地速度（横方向の速度を含む。サンプルでは 7.1 m/s）とは一致しない。

6. 二段開傘にするときは、``main_cd_s`` と ``main_altitude`` を **両方** 書く（片方だけはエラー）。

   .. code-block:: toml

      [recovery]
      enabled = true
      cd_s = 0.5                 # ドローグ
      delay = 1.0
      main_cd_s = 8.0            # メイン
      main_altitude = 300.0      # メインの開傘高度 [m]（射点からの高さ）

   ``cd_s`` と ``delay`` のパラシュートがドローグになる。下降して射点からの高さが ``main_altitude`` に達すると、メインが開き、抗力面積が ``main_cd_s`` に **置き換わる**\ （足し合わせない）。
   メインの高度が頂点より高いときは、ドローグと同時にメインが開く（:ref:`IY-O511`）。

   サンプルの ``cd_s`` を 0.5、メインを 8 m\ :sup:`2`、300 m にした結果は次のとおりである（部品積み上げ法、風 4 m/s）。

   .. code-block:: text

      deploy          27.10 s, 28.5 m/s, q 0.32 kPa
      main deploy     300.0 m at 257.87 s, 16.3 m/s, q 0.16 kPa
      landing         331.9 s, E -681.7 m, N -3106.4 m (34.706698, 139.412959), 4.8 m/s

   期待する結果：``main deploy`` の行が出て、メインの開傘の高度、時刻、そのときの対気速度と動圧が分かる。
   単一のパラシュート（サンプル）では、着地が 685.0 s 後で着地点が北 −7707.7 m、二段開傘では 331.9 s 後で北 −3106.4 m と、ドローグで速く降りた分だけ流される距離が短くなる。

7. 降下方式を切り替えて確かめる。``sim`` の ``--descent`` で、設定を書き換えずに降下方式を選べる。

   .. code-block:: sh

      ignisyeet sim sample.toml --descent ballistic
      ignisyeet sim sample.toml --descent parachute

   サンプルの弾道落下では、着地が 61.9 s 後、着地点が東 −1072.9 m、北 372.3 m で、着地速度は 178.2 m/s である。
   回収系が無効（``enabled = false``）なのに ``--descent parachute`` を指定すると、``error[IY-O001]`` になる。
   ``--descent`` を省くと、``recovery.enabled`` が真ならパラシュート、偽なら弾道落下になる。

   期待する結果：結果パネルの見出し（``Flight (Parachute, …)`` または ``Flight (Ballistic, …)``）が、選んだ降下方式になる。

確認
------

結果パネルまたは ``summary.json`` で次を確かめる。

* ランチャ離脱速度（``rail_exit_speed``）が 15 m/s 以上である。
* パラシュートが開いたときの対気速度が 50 m/s 以下、動圧が 2 kPa 以下である（``drogue_deploy_speed``、``drogue_deploy_q``。二段開傘ではメインの値も）。
  超えると、パラシュートが壊れるおそれがあるとして警告する（:ref:`IY-O518`）。
  サンプルの風を 12 m/s にすると、開傘のときの対気速度は 60 m/s、動圧は 1.5 kPa になり、この警告が出る。
* 着地速度（``landing_speed``）が 15 m/s 以下である。超えると警告する（:ref:`IY-O520`）。
  着地までに開傘しなかったときも警告する（:ref:`IY-O519`）。
* 結果の使えるかどうかの確認は :ref:`page-howto-read-results` にある。

うまくいかないとき
--------------------

``error[IY-O001]: --descent parachute needs recovery.enabled = true``
   回収系が無効のままパラシュート降下を指定した。``[recovery]`` の ``enabled`` を ``true`` にするか、``--descent ballistic`` を使う。

``main_cd_s`` と ``main_altitude`` のどちらか一方だけ書いた
   二段開傘は両方を書くか、両方とも省く（:ref:`IY-S074`）。

``error[IY-S080]: rocket never left the launch rail``
   推力が機体の重さに負けて、レールを離れずに滑り落ちた。モータ、質量、レール長を確かめる（:ref:`page-howto-motor`）。

``warning[IY-O504]``、``INCOMPLETE RESULTS``
   ``sim.max_time`` までに着地しなかった。降下方式とパラシュートの設定を確かめるか、``sim.max_time`` を大きくする。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-reference-config-general`：``[launch]`` と ``[recovery]`` の全項目
* :ref:`page-theory-descent`：降下方式の選び方、開傘の遅れの意味、二段開傘のモデル
* :ref:`page-theory-dynamics`：ランチャ滑走と自由飛行の運動方程式
* :ref:`page-howto-environment`：風の設定
