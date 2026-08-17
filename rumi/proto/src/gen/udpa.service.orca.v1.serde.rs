// @generated
impl serde::Serialize for OrcaLoadReportRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.report_interval.is_some() {
            len += 1;
        }
        if !self.request_cost_names.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("udpa.service.orca.v1.OrcaLoadReportRequest", len)?;
        if let Some(v) = self.report_interval.as_ref() {
            struct_ser.serialize_field("reportInterval", v)?;
        }
        if !self.request_cost_names.is_empty() {
            struct_ser.serialize_field("requestCostNames", &self.request_cost_names)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrcaLoadReportRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "report_interval",
            "reportInterval",
            "request_cost_names",
            "requestCostNames",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ReportInterval,
            RequestCostNames,
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
                            "reportInterval" | "report_interval" => Ok(GeneratedField::ReportInterval),
                            "requestCostNames" | "request_cost_names" => Ok(GeneratedField::RequestCostNames),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrcaLoadReportRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct udpa.service.orca.v1.OrcaLoadReportRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrcaLoadReportRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut report_interval__ = None;
                let mut request_cost_names__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ReportInterval => {
                            if report_interval__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reportInterval"));
                            }
                            report_interval__ = map_.next_value()?;
                        }
                        GeneratedField::RequestCostNames => {
                            if request_cost_names__.is_some() {
                                return Err(serde::de::Error::duplicate_field("requestCostNames"));
                            }
                            request_cost_names__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(OrcaLoadReportRequest {
                    report_interval: report_interval__,
                    request_cost_names: request_cost_names__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("udpa.service.orca.v1.OrcaLoadReportRequest", FIELDS, GeneratedVisitor)
    }
}
