.. _page-config:

設定ファイル
============

シミュレーションの入力は `TOML <https://toml.io/ja/v1.0.0>`_ 形式の設定ファイル 1 つにまとめる。
ファイルは機体、推進、空力、環境、積分、落下分散といった表（``[wind]`` など）に分かれ、
並びは ``examples/sample.toml`` と同じである。この章は各表のキーを、型と既定値、意味と値の範囲とともに引けるように並べる。
相対パスは設定ファイルが置かれたディレクトリを基準に解釈する。ただし ``--out`` で与える出力先は、実行時のカレントディレクトリを基準にする。

どのキーも、コマンドラインで ``--set 表.キー=値`` として上書きできる（:ref:`sec-cli-overrides`）。
設定ファイルは読み込み時に検査し、不正な値はキーの名前を示してエラーにする。有限でない値（``nan`` や ``inf``）はどの数値のキーでも受け付けない。

長さの単位は m、角度の単位は deg である。キーごとに単位が違うものは、意味の欄に示す。
機体上の位置はすべてノーズ先端から後方へ測る。

.. _cfg-atmosphere:

``[atmosphere]``
----------------

大気のモデルを指定する。この設定は飛翔計算と、摩擦抗力の Reynolds 数（射点高度の大気）に使われる。
モデルの式と選び方は :ref:`page-theory-atmosphere-gravity` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-atmosphere-model:

       ``atmosphere.model``
          文字列、``"us1976"``
     - 大気モデル。``"us1976"`` は米国標準大気 1976、``"constant"`` は高度によらず一定の大気。
   * - .. _cfg-atmosphere-temperature_offset:

       ``atmosphere.temperature_offset``
          数値、``0.0``
     - 標準大気の温度に一律に足す温度差 [K]。``us1976`` のみ。有限で、-186.946 より大きい。
   * - .. _cfg-atmosphere-density:

       ``atmosphere.density``
          数値、``1.225``
     - 密度 [kg/m\ :sup:`3`]。``constant`` のみ。有限で正。
   * - .. _cfg-atmosphere-sound_speed:

       ``atmosphere.sound_speed``
          数値、``340.29``
     - 音速 [m/s]。``constant`` のみ。有限で正。
   * - .. _cfg-atmosphere-viscosity:

       ``atmosphere.viscosity``
          数値、``1.789e-5``
     - 粘性係数 [Pa s]。``constant`` のみ。有限で正。

.. raw:: latex

   \renewcommand{\arraystretch}{1}

.. _cfg-wind:

``[wind]``
----------

風を指定する。``profile`` 以外のモデルは、基準高度 ``ref_height`` での風速 ``speed`` と風向 ``direction_deg`` から、高度分布の式で上空の風を決める。
``profile`` は高度別の表から風を決めるので、``speed`` と ``direction_deg`` は使わない（書いてあると警告する）。
モデルの式と選び方は :ref:`page-theory-wind` にある。

.. raw:: latex

   \renewcommand{\arraystretch}{1.35}

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - キー　型・既定値
     - 意味と値の範囲
   * - .. _cfg-wind-model:

       ``wind.model``
          文字列、``"power"``
     - 風速の高度分布。``"constant"`` は一定風、``"power"`` はべき法則、``"log"`` は対数則、``"profile"`` は高度別の表。
   * - .. _cfg-wind-speed:

       ``wind.speed``
          数値、``4.0``
     - 基準高度での風速 [m/s]。0 以上。
   * - .. _cfg-wind-direction_deg:

       ``wind.direction_deg``
          数値、``0.0``
     - 風が吹いてくる方位 [deg]（北から時計回り）。0 以上 360 以下。
   * - .. _cfg-wind-ref_height:

       ``wind.ref_height``
          数値、``2.0``
     - 風速の基準高度 [m]。正。
   * - .. _cfg-wind-exponent:

       ``wind.exponent``
          数値、``6.0``
     - べき法則の指数 :math:`n`。``power`` のみ。正。
   * - .. _cfg-wind-roughness_length:

       ``wind.roughness_length``
          数値、``0.03``
     - 粗度長 :math:`z_0` [m]。``log`` のみ。0 より大きく、``ref_height`` の半分より小さい。
   * - .. _cfg-wind-ground_blend_height:

       ``wind.ground_blend_height``
          数値、``1.0``
     - 地面付近で風速を 0 になめらかにつなぐ高さ [m]。``power`` と ``log`` のみ。正で、``ref_height`` より小さい。``log`` では :math:`z_0` より大きい必要があり、:math:`z_0` 以下なら :math:`\max(\text{値}, 2 z_0)` に読み替える。
   * - .. _cfg-wind-profile:

       ``wind.profile``
          パス、なし
     - 高度別の風の CSV ファイル。``model = "profile"`` のとき必須。
   * - .. _cfg-wind-profile_mean_speed:

       ``wind.profile_mean_speed``
          数値、なし
     - 表の平均風速 :math:`W` をこの値 [m/s] にするよう、表のすべての風速を同じ倍率で拡大・縮小する。``profile`` のみ。有限で 0 以上。省略すると表のまま。
   * - .. _cfg-wind-profile_mean_direction_deg:

       ``wind.profile_mean_direction_deg``
          数値、なし
     - 表の平均風の風向 :math:`\Psi` をこの方位 [deg] にするよう、表のすべての風向を同じ角度だけ回す。``profile`` のみ。0 以上 360 以下。省略すると表のまま。

.. raw:: latex

   \renewcommand{\arraystretch}{1}
