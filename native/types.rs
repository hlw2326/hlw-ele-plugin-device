use napi_derive::napi;

/**
 * 完整设备信息
 */
#[napi(object)]
pub struct DeviceInfo {
    /**
     * 硬件设备标识
     */
    pub did: String,
    /**
     * 系统安装标识
     */
    pub uuid: String,
    /**
     * 设备制造厂商
     */
    pub manufacturer: Option<String>,
    /**
     * 设备产品型号
     */
    pub model: Option<String>,
    /**
     * 操作系统标识
     */
    pub os: String,
    /**
     * 系统产品名称
     */
    pub os_name: String,
    /**
     * 内核版本编号
     */
    pub release: String,
    /**
     * 系统内核名称
     */
    pub kernel: String,
    /**
     * 系统主机名称
     */
    pub host: String,
    /**
     * 系统处理架构
     */
    pub arch: String,
    /**
     * 系统运行秒数
     */
    pub uptime: f64,
    /**
     * 处理器型号
     */
    pub cpu: String,
    /**
     * 逻辑核心数量
     */
    pub cores: Option<u32>,
    /**
     * 硬件显卡名称
     */
    pub gpu: String,
    /**
     * 物理内存字节
     */
    pub memory: f64,
    /**
     * 可用内存字节
     */
    pub free_memory: Option<f64>,
    /**
     * 系统磁盘字节
     */
    pub disk: f64,
    /**
     * 网卡物理地址
     */
    pub mac: String,
    /**
     * 网卡网络地址
     */
    pub ip: Option<String>,
    /**
     * 网络接口名称
     */
    pub net_name: String,
    /**
     * 网络接口类型
     */
    pub net_type: Option<String>,
    /**
     * 主屏像素宽度
     */
    pub screen_width: u32,
    /**
     * 主屏像素高度
     */
    pub screen_height: u32,
    /**
     * 有效缩放比例
     */
    pub screen_scale: Option<f64>,
    /**
     * 电池存在状态
     */
    pub battery: Option<bool>,
}
