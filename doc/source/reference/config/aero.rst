.. _page-reference-config-aero:

空力の設定
============

手法の選択、係数表の格子、パネル法の設定を指定する。

.. _sec-config-aero:

.. _cfg-aero:

``[aero]``
----------

空力係数の求め方と、飛翔計算に使う係数表の範囲を指定する。判断の材料は :ref:`page-theory-aero-choose` にある。

.. _cfg-aero-method:

``aero.method``
^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"barrowman"``
:値/範囲: ``"barrowman"`` ``"panel"`` ``"cfd"`` ``"table"``
:説明: 空力係数の求め方。``"barrowman"`` は STL から抽出した形状に対する部品積み上げ法、``"panel"`` はパネル法、``"cfd"`` は SU2 による CFD、``"table"`` は外部の係数表の読み込みである。
:関連: :ref:`page-theory-aero-choose`

.. _cfg-aero-table:

``aero.table``
^^^^^^^^^^^^^^

:型: パス
:既定値: なし
:説明: ``method = "table"`` のとき必須。係数表の CSV ファイル。
:関連: :ref:`sec-usage-external-table`、:ref:`sec-aero-table-format`

.. _cfg-aero-mach_min:

``aero.mach_min``
^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上 ``mach_max`` 未満
:説明: 係数表の Mach 数の下限。
:関連: :ref:`page-theory-aero-tables`

.. _cfg-aero-mach_max:

``aero.mach_max``
^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``3``
:値/範囲: ``mach_min`` より大きく 10 以下
:説明: 係数表の Mach 数の上限。
:関連: :ref:`page-theory-aero-tables`

.. _cfg-aero-mach_step:

``aero.mach_step``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.02``
:値/範囲: 正
:説明: 係数表の Mach 数の刻み。格子点の数（Mach 数の数 × 迎角の数）は 10\ :sup:`6` 以下。
:関連: :ref:`page-theory-aero-tables`

.. _cfg-aero-alpha_max_deg:

``aero.alpha_max_deg``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``90``
:値/範囲: 正、180 以下
:説明: 係数表の迎角の上限 [deg]（0 から引く）。表の範囲を超える迎角で引くと、結果は使えないものとして終了ステータス 3 で終わる。
:関連: :ref:`sec-exit-status`

.. _cfg-aero-alpha_step_deg:

``aero.alpha_step_deg``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1``
:値/範囲: 正
:説明: 係数表の迎角の刻み [deg]。
:関連: :ref:`page-theory-aero-tables`

.. _cfg-aero-n_slices:

``aero.n_slices``
^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``600``
:値/範囲: 20 以上 10000 以下
:説明: 形状抽出で STL を切る断面の数。
:関連: :ref:`page-theory-geometry`

.. _cfg-aero-fin_threshold:

``aero.fin_threshold``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.05``
:値/範囲: 正
:説明: フィン判定のしきい値（最大胴体半径に対する比）。
:関連: :ref:`page-theory-geometry`

.. _cfg-aero-extrapolation:

``aero.extrapolation``
^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"linear"``
:値/範囲: ``"linear"`` ``"clamp"``
:説明: 迎角と横滑り角が表の格子の外のときの扱い。``"linear"`` は線形外挿（符号と範囲を制限する）、``"clamp"`` は端の値で一定にする。Mach 数は、この値にかかわらず端の値で一定である。CFD の表の、解いた迎角より大きい行の埋め方には効かない。
:関連: :ref:`sec-aero-interp`、:ref:`sec-cfd-table-build`

.. _cfg-aero-table_kind:

``aero.table_kind``
^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"axisymmetric"``
:値/範囲: ``"axisymmetric"`` ``"general"``
:説明: 飛翔計算に使う係数表の種類。``"axisymmetric"`` は全迎角の軸対称表、``"general"`` は迎角と横滑り角の 6 成分の一般表である。
:関連: :ref:`sec-aero-table`、:ref:`page-theory-aero-tables`

.. _cfg-aero-general_mach_step:

``aero.general_mach_step``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.05``
:値/範囲: 正
:説明: ``table_kind = "general"`` で ``method = "barrowman"`` のときの、一般表の Mach 数の刻み。
:関連: :ref:`sec-aero-table-format`

.. _cfg-aero-panel:

``[aero.panel]``
----------------

``method = "panel"`` のときだけ使う。値は手法にかかわらず読み込み時に検査し、範囲外なら項目名を示してエラーにする。

.. _cfg-aero-panel-body_axial:

``aero.panel.body_axial``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``80``
:値/範囲: 6 以上
:説明: 胴体のパネルの軸方向の分割数。フィン根元の翼弦の分割を含む。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-body_circ:

``aero.panel.body_circ``
^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``48``
:値/範囲: 8 以上
:説明: 胴体のパネルの周方向の分割数。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-fin_chord:

``aero.panel.fin_chord``
^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``16``
:値/範囲: 2 以上 ``body_axial`` − 4 以下
:説明: フィン片面のパネルの翼弦方向の分割数。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-fin_span:

``aero.panel.fin_span``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``10``
:値/範囲: 2 以上
:説明: フィン片面のパネルのスパン方向の分割数。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-wake_length:

``aero.panel.wake_length``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``20.0``
:値/範囲: 有限で正
:説明: フィン後流の長さ（全長に対する倍率）。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-tail_radii:

``aero.panel.tail_radii``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``6.0``
:値/範囲: 0 以上
:説明: 底面を塞ぐ尾部フェアリング（円筒 + 半球）の円筒部の長さ（底面半径の倍数）。0 は開口である。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-subsonic_machs:

``aero.panel.subsonic_machs``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列
:既定値: ``[0, 0.3, 0.5, 0.6, 0.7, 0.8]``
:値/範囲: 1 個以上、各要素は 0 以上 1 未満
:説明: 亜音速のパネル解を計算する Mach 数。この間は pchip で内挿する。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-transonic:

``aero.panel.transonic``
^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列（2 要素）
:既定値: ``[0.8, 1.2]``
:値/範囲: :math:`0<` ``lo`` :math:`<` ``hi``、``hi`` :math:`>1`、``hi`` は ``max(aero.mach_max, 1.2)`` 以下
:説明: 遷音速の補間区間の Mach 数の範囲 ``[lo, hi]``。``lo`` は純粋に亜音速の列の最後の Mach 数、``hi`` は最初の完全に超音速の Mach 数である。
:関連: :ref:`page-theory-panel`

.. _cfg-aero-panel-fin_section:

``aero.panel.fin_section``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"biconvex"``
:値/範囲: ``"biconvex"``
:説明: フィンの断面形状。両凸だけを指定できる。
:関連: :ref:`page-theory-panel`

