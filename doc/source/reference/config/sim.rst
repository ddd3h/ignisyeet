.. _page-reference-config-sim:

飛翔計算の設定
================

.. _sec-config-sim:

.. _cfg-sim:

``[sim]``
---------

数値積分と飛翔計算の出力を指定する。

.. _cfg-sim-integrator:

``sim.integrator``
^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"rk4"``
:値/範囲: ``"rk4"`` ``"rk45"`` ``"dop853"``
:説明: 数値積分法。``"rk4"`` は固定刻みの古典的 Runge–Kutta 法、``"rk45"`` は刻み幅制御付きの Dormand–Prince 5(4) 法、``"dop853"`` は刻み幅制御付きの Dormand–Prince 8(5,3) 法。
:関連: :ref:`sec-integrators`、:ref:`page-theory-integrators`

.. _cfg-sim-attitude:

``sim.attitude``
^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"normalize"``
:値/範囲: ``"normalize"`` ``"lie_group"``
:説明: 姿勢の更新法。``"normalize"`` は四元数を通常の状態量として積分し、各ステップ後に正規化する。``"lie_group"`` は SO(3) 上の Runge–Kutta–Munthe-Kaas 法で、正規化をせずに四元数のノルムが丸め誤差の範囲で 1 に保たれる。どの積分法とも組み合わせられる。
:関連: :ref:`sec-integrators`、:ref:`page-theory-integrators`

.. _cfg-sim-dt:

``sim.dt``
^^^^^^^^^^

:型: 数値
:既定値: ``0.002``
:値/範囲: 1e-5 以上 0.05 以下
:説明: ``rk4`` の時間刻み [s]。``rk45`` と ``dop853`` では最初の刻み幅として使う。``rk4`` では ``max_time / dt`` が 1e8 以下。
:関連: :ref:`IY-C025`、:ref:`IY-C026`

.. _cfg-sim-rtol:

``sim.rtol``
^^^^^^^^^^^^

:型: 数値
:既定値: ``1e-7``
:値/範囲: 正で 1e-3 以下
:説明: ``rk45`` と ``dop853`` の相対許容誤差。
:関連: :ref:`IY-C031`

.. _cfg-sim-atol:

``sim.atol``
^^^^^^^^^^^^

:型: 数値
:既定値: ``1e-6``
:値/範囲: 正で 1e-3 以下
:説明: ``rk45`` と ``dop853`` の絶対許容誤差。
:関連: :ref:`IY-C031`

.. _cfg-sim-max_time:

``sim.max_time``
^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``3600``
:値/範囲: 正
:説明: 計算を打ち切る時刻 [s]。この時刻までに着地しなかった飛翔は着地に失敗したものとして扱い、``sim`` は警告して終了ステータス 3 で終わる。落下分散では、そのケースとサンプルを失敗に数える。
:関連: :ref:`sec-exit-status`

.. _cfg-sim-output_interval:

``sim.output_interval``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.05``
:値/範囲: 1e-4 以上 ``max_time`` 以下。``max_time / output_interval`` は 1e7 以下
:説明: ``trajectory.csv`` に書き出す間隔 [s]。``rk45`` と ``dop853`` では刻み幅の上限でもある（燃焼中とレール上は 0.05 s 以下）。
:関連: :ref:`sec-file-trajectory`、:ref:`IY-C030`

.. _cfg-sim-descent:

``sim.descent``
^^^^^^^^^^^^^^^

:型: 文字列
:既定値: 自動
:値/範囲: ``"ballistic"`` ``"parachute"``
:説明: ``sim`` の降下モード。省略時は ``recovery.enabled`` が ``true`` なら ``"parachute"``、``false`` なら ``"ballistic"``。``"parachute"`` は ``recovery.enabled = true`` のときだけ指定できる。
:関連: :ref:`page-theory-descent`、:ref:`IY-C016`

.. _cfg-sim-quasi_steady:

``sim.quasi_steady``
^^^^^^^^^^^^^^^^^^^^

:型: 真偽値
:既定値: ``false``
:値/範囲: ``true`` ``false``
:説明: ``true`` にすると、加速度と回転による空気力のうち、見かけの質量の遅れと対流の回転の項、Magnus 力とモーメント、回転による法線力を切って飛ばす。静的な係数、減衰のモーメント、ロールの項、誘導ロールは残る。設定の一覧に ``aerodynamics: quasi-steady only`` の行が出る。
:関連: :ref:`sec-aero-unsteady`

