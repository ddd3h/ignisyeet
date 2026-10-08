.. _page-howto-environment:

風・大気・地球を設定する
==========================

飛翔の環境を設定する。地上の風だけを与える方法、高度別の風の表（CSV）を作って使う方法、表の風全体を変える方法、地球と大気のモデルの指定を扱う。
どのモデルを選ぶかの判断は :ref:`page-theory-wind` と :ref:`page-theory-atmosphere-gravity` にある。

目的
------

* 打上げ当日の風（地上風、または予報やゾンデの高度別の風）を計算に入れる。
* 表の風を、平均風速と風向を指定して変える。
* 地球・大気のモデルを設定する。

前提
------

* 機体、モータ、射点の設定ができている（:ref:`page-howto-own-rocket`、:ref:`page-howto-motor`、:ref:`page-howto-launcher-parachute`）。
* 風の観測値または予報。地上風だけなら、風速と風向（風が **吹いてくる** 方位）で足りる。

手順
------

1. 地上風だけで計算する（``constant``、``power``、``log``）。

   .. code-block:: toml

      [wind]
      model = "power"        # "constant" | "power" | "log"
      speed = 4.0            # 基準高度での風速 [m/s]
      direction_deg = 0.0    # 風が吹いてくる方位。北から時計回り [deg]（270 は西風）
      ref_height = 2.0       # 基準高度 [m]

   ``speed`` は基準高度 ``ref_height``\ （既定 2 m）での風速である。``power`` と ``log`` は、この風速を上空へ外挿する。
   高度 1 km を超える飛翔では、上空の風が外挿に依存するため、頂点の高さを警告する（:ref:`IY-O514`）。
   サンプルは頂点が 4440 m なので、この警告が毎回出る。結果の使い方は :ref:`page-howto-read-results` にある。
   項目の一覧は :ref:`page-reference-config-environment` にある。

   期待する結果：``sim`` の結果パネルの見出しの下に ``wind 4.0 m/s from 0 deg`` と出る。

2. 高度別の風の表を作る。予報（GFS、ECMWF など）やラジオゾンデの風は、CSV にして使う。
   1 行目は次の見出しでなければならない（大文字小文字と前後の空白は無視する）。空行と ``#`` で始まる行は読み飛ばす。

   .. code-block:: text

      altitude_m,speed,direction_deg
      0,3.0,270
      100,6.0,270
      300,9.0,285
      600,12.0,300
      1000,13.0,310

   各列は、地上高度 [m]（射点からの高さ。0 以上で厳密に増加）、風速 [m/s]（0 以上）、風が吹いてくる方位 [deg]（北から時計回り。0 以上 360 以下）である。
   予報やゾンデのデータから表にするときは、次のように換算する。

   * 高度が海抜の値なら、``launch.altitude`` を引いて射点からの高さにする。引いた高度が負になる行は捨て、射点の高さ（0 m）の行を加える。
     その行には、捨てた行のうち射点に最も近い風か、射点の地上風を入れる。
   * 気圧面ごとのデータは、同じ製品のジオポテンシャル高度を高度の列に使う。
   * 風速が knot なら 0.5144 を掛けて m/s にする（1 kn = 0.5144 m/s）。
   * 東西成分 :math:`u`\ （東向き正）と南北成分 :math:`v`\ （北向き正）で与えられるなら、風速は :math:`\sqrt{u^2+v^2}`、風が吹いてくる方位は :math:`\mathrm{atan2}(-u,\,-v)`\ [deg] を 0° 以上 360° 未満にした値である。
     西風（:math:`u>0,\ v=0`）は 270° になる。
   * 表は、予想される頂点より上まで続ける。表の上端より高いところは上端の風で一定になるので、足りないと、飛翔の上部で上端の風が続くものとして計算される（:ref:`IY-O512`）。

   期待する結果：表の高度が上に向かって増える行だけが並ぶ。付属の ``wind_profile_example.csv`` は、書式を示す架空の表で、実測値ではない。

3. 設定ファイルで表を指定する。

   .. code-block:: toml

      [wind]
      model = "profile"
      profile = "wind_profile_example.csv"   # 設定ファイルのあるディレクトリからの相対パス

   ``profile`` では、``speed`` と ``direction_deg`` を使わない。サンプルの設定ファイルのようにこの 2 つが書いてあると、使わないことを知らせる警告が出る。
   自分の設定ファイルで ``profile`` を使うときは、``[wind]`` からこの 2 つを消しておく。

   ``--set`` で試すときは、設定ファイルを書き換えずに次のようにする（サンプルの設定ファイルでは、警告が毎回出る）。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.model=profile --set wind.profile=wind_profile_example.csv

   期待する結果：サンプルでは、次の行が出る。

   .. code-block:: text

      warning[IY-C502]: wind.speed is ignored for wind.model = "profile"; use wind.profile_mean_speed to scale the profile
      warning[IY-C503]: wind.direction_deg is ignored for wind.model = "profile"; use wind.profile_mean_direction_deg to rotate the profile
      Flight (Parachute, wind 11.9 m/s from 220 deg)
      ...
        apogee          4478.3 m at 26.22 s
      ...
        landing         690.1 s, E -272.4 m, N 4384.1 m (34.774219, 139.417425), 6.8 m/s
        profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)

   ``Flight`` の行の風は、表全体の代表風である。代表風速 :math:`W` は表の最下行から最上行までの平均風速、代表風向 :math:`\Psi` は同じ範囲で平均した風速ベクトルが吹いてくる方位である。
   ``summary.json`` の ``wind_speed`` と ``wind_direction_deg`` にも同じ値が残る。
   最後の行は、表を ``wind.ref_height`` での風に直した値である。

4. 表全体の風速と風向を変える。
   表の形（高度ごとの風速の比と風向のねじれ）を保ったまま、代表風だけを変えるには、``profile_mean_speed`` と ``profile_mean_direction_deg`` を書く。
   表の風速はすべて同じ比で拡大・縮小され、風向はすべて同じ角度だけ回る。

   .. code-block:: sh

      ignisyeet sim sample.toml --set wind.model=profile --set wind.profile=wind_profile_example.csv \
          --set wind.profile_mean_speed=15.6 --set wind.profile_mean_direction_deg=270

   期待する結果：``Flight`` の行が ``wind 15.6 m/s from 270 deg`` になる。サンプルでは、頂点 4495.4 m、着地点が東 4101.2 m、北 3306.8 m である（部品積み上げ法）。
   片方だけを指定すると、もう片方は表のままである。落下分散の格子で最も遠くなった風を ``sim`` で確かめるときに使う（:ref:`page-howto-keepout-zone`）。

   ほぼ無風の表（平均風速が 0.1 m/s 未満）は拡大・回転できず、エラーになる（:ref:`IY-S050`）。
   表の拡大率が 3 を超える、または 1/3 を下回ると、上空の風が非現実的になるため警告する（:ref:`IY-O521`）。

5. 地球と大気を設定する。

   .. code-block:: toml

      [earth]
      model = "flat"             # "flat" | "ecef"
      gravity = "inverse_square" # "constant" | "inverse_square" | "j2"

      [atmosphere]
      model = "us1976"           # "us1976" | "constant"
      temperature_offset = 0.0   # 標準大気の温度に足す差 [K]（us1976 のみ）

   * ``flat`` は射点の接平面の座標で計算し、``ecef`` は WGS84 楕円体に固定した回転座標系で計算する（コリオリ力と遠心力を含む）。``gravity = "j2"`` は ``ecef`` のときだけ指定できる（:ref:`IY-S054`）。
   * 大気は、標準大気（``us1976``）が既定である。当日の気温が標準と違うときは ``temperature_offset`` で温度の差を与える。
     ``constant`` では ``density``、``sound_speed``、``viscosity`` を与える（すべて正）。
   * どちらを選ぶかは、射点からの最大距離と許容誤差で決まる（:ref:`page-theory-atmosphere-gravity`）。

   期待する結果：``sim`` がエラーなく終わり、設定パネルの地球・大気の行が指定どおりになる。

確認
------

* 結果パネルの風が、想定した代表風になっている。
* 警告が出ていない（出た警告の原因が分かっている）。主な警告は次のとおりである。

  * 頂点が表の上端より高い（:ref:`IY-O512`）。表を頂点より上まで延ばす。
  * 表の最下行が地上 0 m より上にある（:ref:`IY-O513`）。0 m の行を加える。
  * ``power`` と ``log`` で頂点が 1 km を超える（:ref:`IY-O514`）。実際の予測には ``profile`` を使う。
  * ``profile`` で使わない ``wind.speed`` と ``wind.direction_deg`` が書いてある（:ref:`IY-C502`、:ref:`IY-C503`）。

うまくいかないとき
--------------------

``error[IY-S034]: cannot load wind profile …``
   表の行に誤りがある。``Caused by:`` の最後の行に、原因と該当の行が出る。
   たとえば風向が 400 だと ``wind profile direction must be between 0 and 360 degrees (row "100,6.0,400")``、高度が増えていないと ``wind profile altitudes must be strictly ascending (row "0,6.0,270")``\ （このときのコードは :ref:`IY-S036`）と出る。
   見出し、行の形、値の範囲、高度の順序の誤りは、それぞれ別のコード（:ref:`IY-S030` から :ref:`IY-S037`）になる。

``wind profile is calm``
   表の平均風速が小さすぎて拡大できない（:ref:`IY-S050`）。風のある表を使う。

``wind.profile (CSV path) is required``\ （:ref:`IY-S045`）
   ``model = "profile"`` で ``profile`` を書いていない。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-reference-config-environment`：``[earth]`` ``[atmosphere]`` ``[wind]`` の全項目
* :ref:`page-theory-wind`：風モデルの式、選び方、基準風の変更の定義
* :ref:`page-theory-atmosphere-gravity`：大気と重力のモデル、地球モデルの選び方
* :ref:`page-howto-keepout-zone`：風を変えて保安距離を検討する手順
