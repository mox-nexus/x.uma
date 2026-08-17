// @generated
impl serde::Serialize for OrcaLoadReport {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.cpu_utilization != 0. {
            len += 1;
        }
        if self.mem_utilization != 0. {
            len += 1;
        }
        if self.rps != 0 {
            len += 1;
        }
        if !self.request_cost.is_empty() {
            len += 1;
        }
        if !self.utilization.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("udpa.data.orca.v1.OrcaLoadReport", len)?;
        if self.cpu_utilization != 0. {
            struct_ser.serialize_field("cpuUtilization", &self.cpu_utilization)?;
        }
        if self.mem_utilization != 0. {
            struct_ser.serialize_field("memUtilization", &self.mem_utilization)?;
        }
        if self.rps != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rps", ToString::to_string(&self.rps).as_str())?;
        }
        if !self.request_cost.is_empty() {
            struct_ser.serialize_field("requestCost", &self.request_cost)?;
        }
        if !self.utilization.is_empty() {
            struct_ser.serialize_field("utilization", &self.utilization)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrcaLoadReport {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "cpu_utilization",
            "cpuUtilization",
            "mem_utilization",
            "memUtilization",
            "rps",
            "request_cost",
            "requestCost",
            "utilization",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CpuUtilization,
            MemUtilization,
            Rps,
            RequestCost,
            Utilization,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "cpuUtilization" | "cpu_utilization" => Ok(GeneratedField::CpuUtilization),
                            "memUtilization" | "mem_utilization" => Ok(GeneratedField::MemUtilization),
                            "rps" => Ok(GeneratedField::Rps),
                            "requestCost" | "request_cost" => Ok(GeneratedField::RequestCost),
                            "utilization" => Ok(GeneratedField::Utilization),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrcaLoadReport;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct udpa.data.orca.v1.OrcaLoadReport")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrcaLoadReport, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut cpu_utilization__ = None;
                let mut mem_utilization__ = None;
                let mut rps__ = None;
                let mut request_cost__ = None;
                let mut utilization__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CpuUtilization => {
                            if cpu_utilization__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cpuUtilization"));
                            }
                            cpu_utilization__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MemUtilization => {
                            if mem_utilization__.is_some() {
                                return Err(serde::de::Error::duplicate_field("memUtilization"));
                            }
                            mem_utilization__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Rps => {
                            if rps__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rps"));
                            }
                            rps__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RequestCost => {
                            if request_cost__.is_some() {
                                return Err(serde::de::Error::duplicate_field("requestCost"));
                            }
                            request_cost__ = Some(
                                map_.next_value::<std::collections::HashMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                        GeneratedField::Utilization => {
                            if utilization__.is_some() {
                                return Err(serde::de::Error::duplicate_field("utilization"));
                            }
                            utilization__ = Some(
                                map_.next_value::<std::collections::HashMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                    }
                }
                Ok(OrcaLoadReport {
                    cpu_utilization: cpu_utilization__.unwrap_or_default(),
                    mem_utilization: mem_utilization__.unwrap_or_default(),
                    rps: rps__.unwrap_or_default(),
                    request_cost: request_cost__.unwrap_or_default(),
                    utilization: utilization__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("udpa.data.orca.v1.OrcaLoadReport", FIELDS, GeneratedVisitor)
    }
}
