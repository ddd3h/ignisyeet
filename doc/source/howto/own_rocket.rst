.. _page-howto-own-rocket:

自分の機体を計算する
======================

自分の機体の STL と質量特性を設定ファイルに書き、``ignisyeet geom`` で形状の抽出結果を確かめる。
ここまでできれば、空力係数の計算と飛翔の計算に進める。

目的
------

* 自分の機体の設定ファイル（TOML）を作る。
* STL から抽出された機体の形状（全長、基準直径、ノーズ、フィン）が、実機と合っていることを確かめる。

前提
------

* :ref:`page-tutorial` を終えている（``ignisyeet sim sample.toml`` が動く）。
* 機体の STL ファイル（ASCII でもバイナリでもよい）。機体の外形を表す **閉じた表面** で、1 つのソリッドとして書き出したものを使う。
* 乾燥質量（推進剤を除いた質量。空のモータケースを含む）、乾燥重心の位置、慣性モーメント。CAD で求めた値を使う。CAD がないときの見積もりは :ref:`sec-usage-inertia-estimate` にある。
* モータの推力曲線（:ref:`page-howto-motor`）と、射点の情報（:ref:`page-howto-launcher-parachute`）。

手順
------

1. 設定ファイルを作る。付属のサンプルをコピーし、自分の機体のディレクトリに STL を置く。

   .. code-block:: sh

      cp ignisyeet-examples/sample.toml rocket.toml
      cp /path/to/my_rocket.stl .

   ``rocket.toml`` の ``[rocket]`` の ``stl`` を自分の STL に変える。相対パスは設定ファイルのあるディレクトリから解釈する。

   .. code-block:: toml

      [rocket]
      stl = "my_rocket.stl"

   期待する結果：``rocket.toml`` の ``[rocket]`` が自分の STL を指している。

2. STL の単位に合わせて ``stl_scale`` を決める。
   STL は長さの単位の情報を持たないので、m に直す係数を :ref:`rocket.stl_scale <cfg-rocket-stl_scale>` で与える。
   mm で作った STL は ``0.001``、m で作った STL は ``1.0`` である（サンプルは mm で ``0.001``）。

   .. code-block:: toml

      stl_scale = 0.001   # STL が mm のとき

   期待する結果：手順 5 の ``geom`` で表示される全長と直径が、実機の寸法（m）と一致する。

3. ノーズの向きを確かめる。
   :ref:`rocket.nose_direction <cfg-rocket-nose_direction>` の既定は ``"auto"`` で、STL の座標のどちらの向きがノーズかを形状から決める。
   自動の判定が怪しいときは、手順 5 で警告が出る（:ref:`IY-G503`、:ref:`IY-G504`、:ref:`IY-G509`）。
   そのときは、STL の座標でノーズが向いている方向を ``"+x"`` ``"-x"`` ``"+y"`` ``"-y"`` ``"+z"`` ``"-z"`` のいずれかで指定する。

   .. code-block:: toml

      nose_direction = "+z"

   期待する結果：手順 5 で、ノーズの長さが妥当な値（直径の 0.4 倍から 8 倍）で表示され、向きの警告が出ない。

4. 質量特性を書く。すべて、機体上の位置はノーズ先端から後方へ測る。

   .. code-block:: toml

      dry_mass = 8.0      # kg。空のモータケースを含み、推進剤は含まない
      cg_dry = 0.85       # m。ノーズ先端から後方へ測った、乾燥重心の位置
      ixx_dry = 0.02      # kg m^2。ロール慣性モーメント（乾燥重心まわり）
      iyy_dry = 1.5       # kg m^2。ピッチ慣性モーメント（izz_dry を省くとヨーも同じ値）

   項目の意味と単位は :ref:`rocket.dry_mass <cfg-rocket-dry_mass>`、:ref:`rocket.cg_dry <cfg-rocket-cg_dry>`、:ref:`rocket.ixx_dry <cfg-rocket-ixx_dry>`、:ref:`rocket.iyy_dry <cfg-rocket-iyy_dry>` にある。
   重心が機軸からずれているとき、ヨーとピッチの慣性モーメントが異なるとき、慣性乗積があるときは、続けて ``cg_lateral_dry``、``izz_dry``、``ixy_dry`` などを書く（設定の一覧は :ref:`page-reference-config-general`）。
   慣性乗積の符号は、CAD の表示と逆になることが多い（:ref:`sec-config-rocket`）。

   期待する結果：``ignisyeet sim`` の設定パネルに、``dry_mass`` と ``cg_dry`` が書いた値で表示される。

5. 形状を抽出して確かめる。

   .. code-block:: sh

      ignisyeet geom rocket.toml

   サンプルでは、次のように表示される。

   .. code-block:: text

      Geometry
        length          1.5000 m
        diameter        0.1000 m (ref area 0.007854 m^2)
        nose            length 0.2913 m, joint angle 1.47 deg, volume ratio 0.529
        fins            4 x root 0.1991 tip 0.0998 span 0.1000 sweep 0.0799 t 0.0040 at x 1.3000 m
        -> out/geometry.json
        -> out/profile.csv

   期待する結果：全長、直径、フィンの枚数と寸法（根元翼弦、翼端翼弦、スパン、後退長、厚さ）が実機と合う。
   ボートテールのある機体では ``boattail`` の行も出る。警告があれば、結果の後ろに ``warning[IY-G…]`` の行が出る（手順 7）。

6. 抽出した断面を図で確かめる。

   .. code-block:: sh

      ignisyeet-plot geometry out

   期待する結果：``out/plots/geometry.png`` ができる（:numref:`fig-howto-geometry-profile`）。
   胴体の半径の分布（ノーズからボートテールまで）とフィンの高さが、機体の外形と一致していることを見る。

   .. _fig-howto-geometry-profile:

   .. figure:: /_generated/plots/geometry_profile.*
      :width: 100%
      :alt: サンプル機体の抽出結果

      サンプル機体で抽出した胴体半径とフィン。破線は設計値で、見るのは抽出結果（実線）が設計値に重なっていることである。

   フィンの検出が複雑な形状（エアブレーキ、カメラポッド、非対称な突起など）で外れたときは、``[rocket.fin_override]`` に正しい値を書く。書いた値が検出結果より優先される。

   .. code-block:: toml

      [rocket.fin_override]
      count = 4
      root_chord = 0.20
      tip_chord = 0.10
      span = 0.10
      sweep = 0.08
      thickness = 0.004
      x_le_root = 1.30

   必要な項目だけ書けばよい（:ref:`rocket.fin_override <cfg-rocket-fin_override-count>`）。

7. 警告を確かめる。
   抽出した形状に不自然な点があると、警告を ``geom``、``aero``、``sim`` の実行時に表示し、``geometry.json`` の ``warnings`` にも残す。
   疑わしい点がなければ ``warnings`` は空である。警告が出たときは、下の「抽出結果の警告」で原因を調べる。

   期待する結果：警告が出ないか、出た警告の原因が分かっていて、結果に影響しないと判断できる。


.. _sec-geometry-warnings:

抽出結果の警告
----------------

:math:`L` を全長、:math:`d` を基準直径とする。

.. list-table::
   :header-rows: 1
   :widths: 38 32 30

   * - 条件
     - 疑うこと
     - コード
   * - :math:`L/d` が 3 未満または 40 を超える
     - 尺度と単位（``stl_scale``）の誤り
     - :ref:`IY-G505`
   * - ノーズ長が :math:`0.4\,d` 未満または :math:`8\,d` を超える
     - ノーズの向きの誤り、またはノーズの検出の誤り
     - :ref:`IY-G506`
   * - 底面半径が基準半径の 60 % 未満
     - 突き出したノズル、または強いボートテールがある。底面抗力はこの半径で計算する
     - :ref:`IY-G508`
   * - 底面半径が基準半径の 1.05 倍を超える
     - フィンが胴体の底面より後ろへ突き出している
     - :ref:`IY-G507`
   * - フィンの後縁が機体の後端より後ろにある
     - フィンの位置、または ``fin_override`` の ``x_le_root`` の誤り
     - :ref:`IY-G512`
   * - フィンの根元前縁が全長の前半にある
     - 機体が逆向き（``nose_direction`` を確認する）
     - :ref:`IY-G509`
   * - フィンの根元の中央が全長の 60 % より前にある
     - フィンが異例に前方にある
     - :ref:`IY-G510`
   * - フィンのスパンが基準直径の 2 倍を超える
     - スパンの検出の誤り
     - :ref:`IY-G511`
   * - 短い突起が基準半径の決定から除かれた
     - ランチラグやレールボタンの突起を除いて基準半径を決めた
     - :ref:`IY-G502`
   * - ノーズの向きを端の半径だけで決めた、または、端の形とフィンの位置が逆の向きを示した
     - ``nose_direction`` で向きを指定する
     - :ref:`IY-G504`、:ref:`IY-G503`
   * - 表面に開いた辺（辺の 1 % 以下）や、3 つ以上の三角形に共有される辺がある
     - 小さなメッシュの欠陥。抽出結果を確かめる
     - :ref:`IY-G501`

表面が閉じていない（穴がある、殻だけの面である）ときは、警告ではなくエラーになる。開いた辺が辺の 1 % を超えるとき（:ref:`IY-G010`）と、穴の幅が基準直径の 0.1 倍を超えるとき（:ref:`IY-G011`）である。
穴をふさぐか、ソリッドとして書き出し直す。

尺度の誤りは、機体の大きさの検査でも見つかる。``stl_scale`` を掛けた全長が 0.1 m から 20 m の範囲の外にあるとエラー（:ref:`IY-O005`）、直径が 0.005 m から 3 m の範囲の外にあると警告（:ref:`IY-O503`）になる。
乾燥重心が機体の全長より後方にあるときと、モータが機体に収まらないときもエラーになる（:ref:`IY-O006`、:ref:`IY-O007`、:ref:`IY-O008`）。

確認
------

* ``ignisyeet geom rocket.toml`` が終了ステータス 0 で終わる。
* 表示された全長、直径、フィンの枚数と寸法が、実機または CAD の寸法と合う。
* ``out/geometry.json`` の ``warnings`` が空である。
* ``out/geometry.json`` の ``body_x`` ``body_y`` ``body_z`` は、機体座標の軸の向きを STL の座標で表した単位ベクトルである。慣性乗積や ``cg_lateral_dry`` を書くときは、この向きで符号を決める（:ref:`sec-body-basis`）。

うまくいかないとき
--------------------

``error[IY-O005]: body length … m is outside the plausible 0.1-20 m range``
   STL の単位と ``stl_scale`` が合っていない。サンプルの STL（mm）に ``stl_scale = 1.0`` を指定すると、全長が 1500 m になりこのエラーで止まる。
   メッセージの末尾に、現在の ``stl_scale`` と、mm を m に直す値 ``0.001`` が出る。

``warning[IY-G506]``、``warning[IY-G508]``、``warning[IY-G509]`` が同時に出る
   ノーズが逆向きに読まれている。サンプルに ``nose_direction = "-z"`` を指定すると、この 3 つが出る。``nose_direction`` を実際の向きに直す。

``error[IY-G010]``、``error[IY-G011]``
   STL の表面が閉じていない。CAD で穴をふさぐか、ソリッドとして書き出し直す。

全長や直径は合うが、フィンの寸法が合わない
   ``[rocket.fin_override]`` で正しい値を与える（手順 6）。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-howto-motor`、:ref:`page-howto-launcher-parachute`：次に設定する項目
* :ref:`page-reference-config-general`：``[rocket]`` の全項目
* :ref:`page-theory-geometry`：形状の抽出が前提とするモデルと適用範囲
* :ref:`sec-usage-inertia-estimate`：CAD がないときの慣性モーメントの見積もり
* :ref:`errors-geometry`：形状のエラーコード
