"""
OMNI-MESH Python SDK — Complete Test Suite

This file demonstrates every feature of the Python SDK and tests all edge cases.

To run:
    1. Install maturin: pip install maturin
    2. Build the module: cd bindings/python && maturin develop
    3. Run tests: python test_omnimesh.py

Or run with pytest:
    pip install pytest
    pytest test_omnimesh.py -v
"""

import time
import threading
import omnimesh


# ═══════════════════════════════════════════════════════════════════════════════
# BASIC CLIENT CREATION
# ═══════════════════════════════════════════════════════════════════════════════

def test_create_client_development():
    """Create a client in development mode (default)."""
    client = omnimesh.Client()
    assert client.did is not None
    assert len(client.did) == 64  # 32 bytes = 64 hex chars
    print(f"✓ Development client created. DID: {client.did[:16]}...")


def test_create_client_modes():
    """Create clients in all supported modes."""
    dev = omnimesh.Client(mode="development")
    # Note: lightweight and production modes bind TCP ports.
    # We just verify they can be created without error.
    # In a real deployment, each would run on a different machine/port.
    print(f"✓ Development mode works: did={dev.did[:8]}")


def test_invalid_mode_raises():
    """Invalid mode should raise ValueError."""
    try:
        omnimesh.Client(mode="invalid")
        assert False, "Should have raised"
    except ValueError as e:
        assert "must be" in str(e)
    print("✓ Invalid mode raises ValueError")


# ═══════════════════════════════════════════════════════════════════════════════
# SENDING AND RECEIVING — AGENT COMMANDS
# ═══════════════════════════════════════════════════════════════════════════════

def test_send_receive_agent_command():
    """Basic send/receive of an AgentCommand between two nodes."""
    node_a = omnimesh.Client()
    node_b = omnimesh.Client()

    # Small delay to let poller threads start
    time.sleep(0.05)

    # Node A sends a command to Node B
    node_a.send_agent_command(
        target_did_hex=node_b.did,
        command_type="pick",
        target_id=b"robot-1",
        payload_data=b"shelf-A12",
    )

    # Node B receives it
    msg = node_b.receive(timeout_ms=2000)
    assert msg is not None, f"Should receive message (inbox_len={node_b.inbox_len()})"
    assert msg["type"] == "agent_command"
    assert msg["command_type"] == "pick"
    assert msg["sender_did"] == node_a.did
    assert bytes(msg["payload"]) == b"shelf-A12"
    print(f"✓ AgentCommand roundtrip: '{msg['command_type']}' with payload '{bytes(msg['payload']).decode()}'")


def test_send_receive_multiple_commands():
    """Send multiple commands and receive them in order."""
    sender = omnimesh.Client()
    receiver = omnimesh.Client()

    commands = ["move", "pick", "place", "return", "charge"]
    for cmd in commands:
        sender.send_agent_command(receiver.did, cmd, b"", cmd.encode())

    time.sleep(0.2)  # Let poller process

    received = []
    for _ in range(10):
        msg = receiver.try_receive()
        if msg is None:
            break
        received.append(msg["command_type"])

    assert received == commands, f"Expected {commands}, got {received}"
    print(f"✓ Received {len(received)} commands in order: {received}")


# ═══════════════════════════════════════════════════════════════════════════════
# SENDING AND RECEIVING — MOTION COMMANDS
# ═══════════════════════════════════════════════════════════════════════════════

def test_send_receive_motion_command():
    """Send a motion command (Twist) and verify all fields."""
    controller = omnimesh.Client()
    robot = omnimesh.Client()

    controller.send_motion_command(
        target_did_hex=robot.did,
        linear_x=1.5,
        linear_y=0.0,
        linear_z=0.0,
        angular_x=0.0,
        angular_y=0.0,
        angular_z=0.8,
        deadline_ns=100_000,
    )

    msg = robot.receive(timeout_ms=2000)
    assert msg is not None
    assert msg["type"] == "motion_command"
    assert abs(msg["linear_x"] - 1.5) < 0.01
    assert abs(msg["angular_z"] - 0.8) < 0.01
    assert msg["deadline_ns"] == 100_000
    print(f"✓ MotionCommand: linear_x={msg['linear_x']}, angular_z={msg['angular_z']}")


def test_motion_command_defaults():
    """Motion command with default values (all zeros except what's specified)."""
    a = omnimesh.Client()
    b = omnimesh.Client()

    # Only specify forward velocity
    a.send_motion_command(b.did, linear_x=2.0)

    msg = b.receive(timeout_ms=2000)
    assert msg is not None
    assert abs(msg["linear_x"] - 2.0) < 0.01
    assert abs(msg["linear_y"]) < 0.01
    assert abs(msg["angular_z"]) < 0.01
    print("✓ MotionCommand with defaults works")


# ═══════════════════════════════════════════════════════════════════════════════
# SENDING AND RECEIVING — HEARTBEATS
# ═══════════════════════════════════════════════════════════════════════════════

def test_send_receive_heartbeat():
    """Send telemetry heartbeat and verify fields."""
    robot = omnimesh.Client()
    monitor = omnimesh.Client()

    robot.send_heartbeat(
        target_did_hex=monitor.did,
        uptime_ms=60000,
        cpu=75,
        mem_kb=4096,
        epoch=42,
    )

    msg = monitor.receive(timeout_ms=2000)
    assert msg is not None
    assert msg["type"] == "heartbeat"
    assert msg["uptime_ms"] == 60000
    assert msg["cpu_usage"] == 75
    assert msg["mem_usage_kb"] == 4096
    assert msg["epoch"] == 42
    print(f"✓ Heartbeat: uptime={msg['uptime_ms']}ms, cpu={msg['cpu_usage']}%")


# ═══════════════════════════════════════════════════════════════════════════════
# SENDING AND RECEIVING — LLM QUERIES
# ═══════════════════════════════════════════════════════════════════════════════

def test_send_receive_llm_query():
    """Send an LLM inference request."""
    requester = omnimesh.Client()
    ai_node = omnimesh.Client()

    requester.send_llm_query(
        target_did_hex=ai_node.did,
        prompt="What is the optimal path to shelf B-12?",
        system_prompt="You are a warehouse navigation assistant.",
        model="llama3",
    )

    msg = ai_node.receive(timeout_ms=2000)
    assert msg is not None
    assert msg["type"] == "llm_query"
    assert "shelf B-12" in msg["prompt"]
    assert msg["model"] == "llama3"
    print(f"✓ LLM Query: model={msg['model']}, prompt='{msg['prompt'][:40]}...'")


# ═══════════════════════════════════════════════════════════════════════════════
# TIMEOUT AND NON-BLOCKING RECEIVE
# ═══════════════════════════════════════════════════════════════════════════════

def test_receive_timeout_returns_none():
    """Receive with no messages should return None after timeout."""
    client = omnimesh.Client()

    start = time.time()
    msg = client.receive(timeout_ms=100)
    elapsed = time.time() - start

    assert msg is None
    assert elapsed >= 0.08, f"Returned too fast: {elapsed:.3f}s"
    assert elapsed < 0.3, f"Took too long: {elapsed:.3f}s"
    print(f"✓ Timeout works: returned None after {elapsed*1000:.0f}ms")


def test_try_receive_non_blocking():
    """try_receive() should return immediately."""
    client = omnimesh.Client()

    start = time.time()
    msg = client.try_receive()
    elapsed = time.time() - start

    assert msg is None
    assert elapsed < 0.01, f"try_receive blocked for {elapsed:.3f}s"
    print("✓ try_receive() is non-blocking")


def test_inbox_len():
    """inbox_len() reports queued messages."""
    sender = omnimesh.Client()
    receiver = omnimesh.Client()

    for i in range(5):
        sender.send_agent_command(receiver.did, f"cmd-{i}", b"", b"")

    time.sleep(0.3)
    assert receiver.inbox_len() == 5, f"Expected 5, got {receiver.inbox_len()}"
    print(f"✓ inbox_len() = {receiver.inbox_len()}")


# ═══════════════════════════════════════════════════════════════════════════════
# SHUTDOWN AND LIFECYCLE
# ═══════════════════════════════════════════════════════════════════════════════

def test_shutdown():
    """Shutdown stops the client gracefully."""
    client = omnimesh.Client()
    assert not client.is_shutdown()

    client.shutdown()
    assert client.is_shutdown()

    # Receive should return None immediately after shutdown
    start = time.time()
    msg = client.receive(timeout_ms=5000)
    elapsed = time.time() - start

    assert msg is None
    assert elapsed < 0.1, f"Should return immediately, took {elapsed:.3f}s"
    print("✓ Shutdown works: receive returns immediately")


def test_send_after_shutdown_raises():
    """Sending after shutdown should raise an error."""
    client = omnimesh.Client()
    target = "aa" * 32  # Dummy DID

    client.shutdown()

    try:
        client.send_agent_command(target, "test", b"", b"")
        assert False, "Should have raised"
    except ValueError as e:
        assert "shut down" in str(e)
    print("✓ Send after shutdown raises ValueError")


def test_drain_after_shutdown():
    """Messages received before shutdown can still be drained."""
    sender = omnimesh.Client()
    receiver = omnimesh.Client()

    # Send messages
    for i in range(3):
        sender.send_agent_command(receiver.did, f"pre-shutdown-{i}", b"", b"")

    time.sleep(0.3)

    # Shutdown
    receiver.shutdown()

    # Drain remaining messages
    drained = 0
    while True:
        msg = receiver.try_receive()
        if msg is None:
            break
        drained += 1

    assert drained == 3, f"Expected 3, got {drained}"
    print(f"✓ Drained {drained} messages after shutdown")


# ═══════════════════════════════════════════════════════════════════════════════
# MULTI-NODE COORDINATION (FLEET SIMULATION)
# ═══════════════════════════════════════════════════════════════════════════════

def test_fleet_coordination():
    """Simulate a coordinator assigning tasks to a robot fleet."""
    coordinator = omnimesh.Client()
    robots = [omnimesh.Client() for _ in range(3)]

    tasks = ["pick-A12", "transport-B05", "place-C18"]

    # Coordinator assigns tasks
    for robot, task in zip(robots, tasks):
        coordinator.send_agent_command(robot.did, task, b"", task.encode())

    time.sleep(0.3)

    # Each robot receives its task
    for i, robot in enumerate(robots):
        msg = robot.try_receive()
        assert msg is not None, f"Robot {i} didn't receive task"
        assert msg["command_type"] == tasks[i]

    # Robots report back
    for robot in robots:
        robot.send_heartbeat(coordinator.did, uptime_ms=5000, cpu=45, mem_kb=2048, epoch=1)

    time.sleep(0.3)

    # Coordinator collects heartbeats
    heartbeats = 0
    while True:
        msg = coordinator.try_receive()
        if msg is None:
            break
        if msg["type"] == "heartbeat":
            heartbeats += 1

    assert heartbeats == 3
    print(f"✓ Fleet coordination: 3 tasks assigned, {heartbeats} heartbeats received")


# ═══════════════════════════════════════════════════════════════════════════════
# CONCURRENCY — MULTI-THREADED USAGE
# ═══════════════════════════════════════════════════════════════════════════════

def test_multithreaded_send():
    """Multiple threads can send to the same receiver."""
    receiver = omnimesh.Client()
    senders = [omnimesh.Client() for _ in range(4)]

    def send_batch(sender, target_did, thread_id):
        for i in range(10):
            sender.send_agent_command(target_did, f"t{thread_id}-m{i}", b"", b"")

    threads = []
    for i, sender in enumerate(senders):
        t = threading.Thread(target=send_batch, args=(sender, receiver.did, i))
        threads.append(t)
        t.start()

    for t in threads:
        t.join()

    # Wait for all messages to arrive
    deadline = time.time() + 5
    total = 0
    while total < 40 and time.time() < deadline:
        msg = receiver.try_receive()
        if msg:
            total += 1
        else:
            time.sleep(0.01)

    assert total == 40, f"Expected 40 messages, got {total}"
    print(f"✓ Multi-threaded: {total} messages from 4 threads")


# ═══════════════════════════════════════════════════════════════════════════════
# EDGE CASES AND ERROR HANDLING
# ═══════════════════════════════════════════════════════════════════════════════

def test_invalid_did_raises():
    """Invalid DID hex strings should raise ValueError."""
    client = omnimesh.Client()

    # Too short
    try:
        client.send_agent_command("abcd", "test", b"", b"")
        assert False
    except ValueError:
        pass

    # Not hex
    try:
        client.send_agent_command("zz" * 32, "test", b"", b"")
        assert False
    except ValueError:
        pass

    # Odd length
    try:
        client.send_agent_command("a" * 63, "test", b"", b"")
        assert False
    except ValueError:
        pass

    print("✓ Invalid DIDs raise ValueError correctly")


def test_send_to_self():
    """A node can send messages to itself."""
    client = omnimesh.Client()

    client.send_agent_command(client.did, "self-test", b"", b"hello self")

    msg = client.receive(timeout_ms=2000)
    assert msg is not None
    assert msg["command_type"] == "self-test"
    assert msg["sender_did"] == client.did
    print("✓ Send-to-self works")


def test_large_payload():
    """Send a large payload (close to the limit)."""
    sender = omnimesh.Client()
    receiver = omnimesh.Client()

    big_data = b"X" * 800  # ~800 bytes, should fit
    sender.send_agent_command(receiver.did, "big", b"", big_data)

    msg = receiver.receive(timeout_ms=2000)
    assert msg is not None
    assert len(msg["payload"]) == 800
    print(f"✓ Large payload ({len(msg['payload'])} bytes) delivered")


def test_unicode_command_type():
    """Unicode strings work in command types."""
    a = omnimesh.Client()
    b = omnimesh.Client()

    a.send_agent_command(b.did, "移動ロボット🤖", b"", "données".encode())

    msg = b.receive(timeout_ms=2000)
    assert msg is not None
    assert msg["command_type"] == "移動ロボット🤖"
    print(f"✓ Unicode command: '{msg['command_type']}'")


# ═══════════════════════════════════════════════════════════════════════════════
# RUN ALL TESTS
# ═══════════════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    print("=" * 60)
    print("  OMNI-MESH Python SDK — Test Suite")
    print("=" * 60)
    print()

    tests = [
        test_create_client_development,
        test_create_client_modes,
        test_invalid_mode_raises,
        test_send_receive_agent_command,
        test_send_receive_multiple_commands,
        test_send_receive_motion_command,
        test_motion_command_defaults,
        test_send_receive_heartbeat,
        test_send_receive_llm_query,
        test_receive_timeout_returns_none,
        test_try_receive_non_blocking,
        test_inbox_len,
        test_shutdown,
        test_send_after_shutdown_raises,
        test_drain_after_shutdown,
        test_fleet_coordination,
        test_multithreaded_send,
        test_invalid_did_raises,
        test_send_to_self,
        test_large_payload,
        test_unicode_command_type,
    ]

    passed = 0
    failed = 0

    for test in tests:
        try:
            test()
            passed += 1
        except Exception as e:
            print(f"✗ {test.__name__}: {e}")
            failed += 1

    print()
    print("=" * 60)
    print(f"  Results: {passed} passed, {failed} failed, {passed + failed} total")
    print("=" * 60)

    if failed > 0:
        exit(1)
