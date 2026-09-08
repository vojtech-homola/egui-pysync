"""Collection behavior independent of showcase action callbacks."""


def test_vector_mutation(server_bundle):
    _, states, _ = server_bundle
    values = states.value_vec.items
    values.set([10, -3, 27], update=True)
    assert values.get() == [10, -3, 27]
    values.add_item(32, update=True)
    assert values.get() == [10, -3, 27, 32]
    values.remove_item(3, update=True)
    assert values.get() == [10, -3, 27]
    values.set([], update=True)
    assert values.get() == []


def test_map_mutation(server_bundle):
    _, states, _ = server_bundle
    values = states.value_map.items
    values.set({1: 100, 2: 200, 5: 500}, update=True)
    assert values.get() == {1: 100, 2: 200, 5: 500}
    values.set_item(6, 600, update=True)
    assert values.get() == {1: 100, 2: 200, 5: 500, 6: 600}
    values.remove_item(1, update=True)
    assert values.get() == {2: 200, 5: 500, 6: 600}
    values.set({}, update=True)
    assert values.get() == {}
