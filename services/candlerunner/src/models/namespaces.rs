use uuid::Uuid;

static mut STRATEGY_INSTANCE_NS: Option<Uuid> = None;
static mut PLACE_ORDER_SETTINGS_NS: Option<Uuid> = None;
static mut PARAMS_SET_NS: Option<Uuid> = None;

pub fn get_strategy_instance_ns() -> &'static Uuid {
    unsafe {
        STRATEGY_INSTANCE_NS
            .get_or_insert_with(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"strategyInstanceId"))
    }
}

pub fn get_place_order_settings_ns() -> &'static Uuid {
    unsafe {
        PLACE_ORDER_SETTINGS_NS
            .get_or_insert_with(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"placeOrderSettings"))
    }
}

pub fn get_params_set_ns() -> &'static Uuid {
    unsafe {
        PARAMS_SET_NS
            .get_or_insert_with(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"paramsSet"))
    }
}
