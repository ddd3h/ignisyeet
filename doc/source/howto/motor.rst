.. _page-howto-motor:

モータを設定する
==================

モータの推力曲線を RASP 形式（``.eng``）のファイルで用意し、設定ファイルの ``[motor]`` に書く。
読み込みの検査と、燃焼時間の確認までを行う。

目的
------

* 推力曲線のファイルを用意して、``sim`` に読み込ませる。
* モータの取り付け位置とノズルの寸法を設定する。

前提
------

* 機体の設定（:ref:`page-howto-own-rocket`）ができている。
* モータの推力曲線。燃焼試験の実測値、または市販モータの公開データ（ThrustCurve.org などで RASP 形式として配布されている）を使う。
  IgnisYeet は推力を計算しない。ファイルの推力履歴をそのまま使う（:ref:`page-theory-propulsion`）。

手順
------

1. 推力曲線を RASP 形式で書く。1 行目から順に、コメント行（``;`` で始まる）、ヘッダ 1 行、「時刻 [s] と推力 [N]」の組の行である。
   付属のサンプルモータ（架空）は次のとおりである。

   .. code-block:: text

      ; Fictional motor for the IgnisYeet example (not a real product).
      ; name  dia[mm] len[mm] delays prop[kg] total[kg] maker
      IY-SAMPLE 75 500 0 3.0 4.5 IgnisYeet
      0.00 0
      0.05 2300
      0.20 2400
      1.00 2300
      2.00 2150
      2.60 2000
      2.90 900
      3.10 0

   ヘッダは、名前、直径 [mm]、長さ [mm]、遅延、推進剤質量 [kg]、全備質量 [kg]、製造者の 7 項目である。
   このうち、直径、長さ、推進剤質量を使う。全備質量は読み込んで検査するだけで、質量には使わない。
   したがって、空のモータケースの質量は ``rocket.dry_mass`` に含める。
   時刻の間は線形に補間し、最後の時刻を燃焼終了の時刻とする。
   先頭が :math:`t=0` でないときは、:math:`(0,0)` を補う。

   期待する結果：1 つのファイルに 1 つのモータが書かれ、時刻が厳密に増加している。

2. 設定ファイルに ``[motor]`` を書く。

   .. code-block:: toml

      [motor]
      eng = "sample_motor.eng"
      # aft_x = 1.5                  # モータ後端の位置 [m]（ノーズ先端から後方へ）。省くと機体の全長
      nozzle_exit_diameter = 0.03    # ノズル出口の直径 [m]。燃焼中はこの面積だけ底面抗力が減る

   ``eng`` は必須で、相対パスは設定ファイルのあるディレクトリから解釈する。
   モータが機体の後端に収まっているなら ``aft_x`` は省ける。
   機体の途中にモータを置くときは、``aft_x`` にモータ後端の位置を書く。
   項目の一覧は :ref:`page-reference-config-general` にある。

   期待する結果：``ignisyeet sim`` が ``.eng`` を読み込み、エラーなく飛翔を計算する。

3. 推進剤質量を上書きするときは ``propellant_mass`` を書く。
   既定ではヘッダの値を使う。ヘッダの値が実測と異なるときだけ指定する。

   .. code-block:: toml

      propellant_mass = 2.95   # kg

   期待する結果：飛翔の初期質量が ``dry_mass + propellant_mass`` になる。

4. 推力線のずれを与えるときは、``misalignment_deg``、``misalignment_azimuth_deg``、``misalignment_offset`` を書く。
   推力の向きが機軸からずれた状態、またはノズル出口の位置が機軸からずれた状態を再現する（既定は 0 で、ずれなし）。
   意味と符号は :ref:`page-reference-config-general` と :ref:`page-theory-propulsion` にある。

5. 飛翔を計算し、燃焼時間を確かめる。

   .. code-block:: sh

      ignisyeet sim rocket.toml
      grep burnout_time out/summary.json

   サンプルの出力は次のとおりである。

   .. code-block:: text

      "burnout_time": 3.1,

   期待する結果：``burnout_time`` が ``.eng`` の最後の時刻と一致する。端末の結果パネルの ``burnout`` の行にも同じ値が出る。

.. _sec-propulsion-eng:

``.eng`` の検査
-----------------

読み込み時に ``.eng`` を検査し、誤りがあればファイル名と行番号を示してエラーにする。疑わしいだけの推力曲線は警告になる。

.. list-table::
   :header-rows: 1
   :widths: 50 50

   * - 症状
     - コード
   * - 見出し行の項目が足りない、数値でない、直径・長さ・質量の値が不正
     - :ref:`IY-S002` から :ref:`IY-S009`
   * - 推力曲線の時刻や推力が不正、時刻が増加していない、全力積が正でない
     - :ref:`IY-S010` から :ref:`IY-S015`
   * - 推力曲線が 1 点だけ
     - :ref:`IY-S500`
   * - 最後の点の推力が 0 N でない
     - :ref:`IY-S501`
   * - 燃焼時間が極端に短い
     - :ref:`IY-S502`

確認
------

* ``burnout_time`` が ``.eng`` の最後の時刻に等しい。
* 結果パネルの ``rail exit`` に、ランチャ離脱の時刻と速度が出る。離脱速度が 15 m/s を下回ると警告が出る（:ref:`IY-O515`）。
* 推力曲線の警告（上の「``.eng`` の検査」の表）が出ていない。

うまくいかないとき
--------------------

``error[IY-S013]: cannot read motor file …``
   ``.eng`` の内容に誤りがある。``Caused by:`` の次の行に、行番号と原因が出る。
   たとえば時刻が増加していないと、``line 6: thrust curve times must strictly increase, got 0 after 0.05`` と出る。
   行番号の行を直す。ヘッダの誤り、数値でない行、負の値は、それぞれ別のコード（:ref:`IY-S002` から :ref:`IY-S015`）になる。

``error[IY-S080]: rocket never left the launch rail (thrust too low?)``
   推力が機体の重さに負けて、レールを離れずに滑り落ちた。推力曲線の単位（N）、機体の質量、レール長（``launch.rail_length``）を確かめる。

``error[IY-O007]`` または ``error[IY-O008]``
   モータが機体より長い、または機体の外にある。``.eng`` のヘッダの長さ [mm]、``rocket.stl_scale``、``motor.aft_x`` を確かめる。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-howto-launcher-parachute`：射点とパラシュートの設定
* :ref:`page-reference-config-general`：``[motor]`` の全項目
* :ref:`page-theory-propulsion`：推力、推進剤の消費、質量特性のモデル
