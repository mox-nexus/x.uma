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
        if self.rps_fractional != 0. {
            len += 1;
        }
        if self.eps != 0. {
            len += 1;
        }
        if !self.named_metrics.is_empty() {
            len += 1;
        }
        if self.application_utilization != 0. {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("xds.data.orca.v3.OrcaLoadReport", len)?;
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
        if self.rps_fractional != 0. {
            struct_ser.serialize_field("rpsFractional", &self.rps_fractional)?;
        }
        if self.eps != 0. {
            struct_ser.serialize_field("eps", &self.eps)?;
        }
        if !self.named_metrics.is_empty() {
            struct_ser.serialize_field("namedMetrics", &self.named_metrics)?;
        }
        if self.application_utilization != 0. {
            struct_ser.serialize_field("applicationUtilization", &self.application_utilization)?;
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
            "rps_fractional",
            "rpsFractional",
            "eps",
            "named_metrics",
            "namedMetrics",
            "application_utilization",
            "applicationUtilization",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CpuUtilization,
            MemUtilization,
            Rps,
            RequestCost,
            Utilization,
            RpsFractional,
            Eps,
            NamedMetrics,
            ApplicationUtilization,
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
                            "rpsFractional" | "rps_fractional" => Ok(GeneratedField::RpsFractional),
                            "eps" => Ok(GeneratedField::Eps),
                            "namedMetrics" | "named_metrics" => Ok(GeneratedField::NamedMetrics),
                            "applicationUtilization" | "application_utilization" => Ok(GeneratedField::ApplicationUtilization),
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
                formatter.write_str("struct xds.data.orca.v3.OrcaLoadReport")
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
                let mut rps_fractional__ = None;
                let mut eps__ = None;
                let mut named_metrics__ = None;
                let mut application_utilization__ = None;
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
                        GeneratedField::RpsFractional => {
                            if rps_fractional__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rpsFractional"));
                            }
                            rps_fractional__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Eps => {
                            if eps__.is_some() {
                                return Err(serde::de::Error::duplicate_field("eps"));
                            }
                            eps__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NamedMetrics => {
                            if named_metrics__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namedMetrics"));
                            }
                            named_metrics__ = Some(
                                map_.next_value::<std::collections::HashMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                        GeneratedField::ApplicationUtilization => {
                            if application_utilization__.is_some() {
                                return Err(serde::de::Error::duplicate_field("applicationUtilization"));
                            }
                            application_utilization__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(OrcaLoadReport {
                    cpu_utilization: cpu_utilization__.unwrap_or_default(),
                    mem_utilization: mem_utilization__.unwrap_or_default(),
                    rps: rps__.unwrap_or_default(),
                    request_cost: request_cost__.unwrap_or_default(),
                    utilization: utilization__.unwrap_or_default(),
                    rps_fractional: rps_fractional__.unwrap_or_default(),
                    eps: eps__.unwrap_or_default(),
                    named_metrics: named_metrics__.unwrap_or_default(),
                    application_utilization: application_utilization__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("xds.data.orca.v3.OrcaLoadReport", FIELDS, GeneratedVisitor)
    }
}
