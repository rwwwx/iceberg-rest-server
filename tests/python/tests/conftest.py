import dataclasses
import os
import urllib
import uuid

import pyiceberg.catalog
import pyiceberg.catalog.rest
import pyiceberg.typedef
import pytest
import requests

ICEBERG_REST_TEST_MANAGEMENT_URL = os.environ.get("ICEBERG_REST_TEST_MANAGEMENT_URL")
ICEBERG_REST_TEST_CATALOG_URL = os.environ.get("ICEBERG_REST_TEST_CATALOG_URL")
ICEBERG_REST_TEST_S3_ACCESS_KEY = os.environ.get("ICEBERG_REST_TEST_S3_ACCESS_KEY")
ICEBERG_REST_TEST_S3_SECRET_KEY = os.environ.get("ICEBERG_REST_TEST_S3_SECRET_KEY")
ICEBERG_REST_TEST_S3_BUCKET = os.environ.get("ICEBERG_REST_TEST_S3_BUCKET")
ICEBERG_REST_TEST_S3_ENDPOINT = os.environ.get("ICEBERG_REST_TEST_S3_ENDPOINT")
ICEBERG_REST_TEST_S3_REGION = os.environ.get("ICEBERG_REST_TEST_S3_REGION", None)
ICEBERG_REST_TEST_S3_PATH_STYLE_ACCESS = os.environ.get(
    "ICEBERG_REST_TEST_S3_PATH_STYLE_ACCESS"
)
ICEBERG_REST_TEST_SPARK_ICEBERG_VERSION = os.environ.get(
    "ICEBERG_REST_TEST_SPARK_ICEBERG_VERSION", "1.5.2"
)


def string_to_bool(s: str) -> bool:
    return s.lower() in ["true", "1"]


def get_storage_config() -> dict:
    if ICEBERG_REST_TEST_S3_BUCKET is None:
        pytest.skip("ICEBERG_REST_TEST_S3_BUCKET is not set")

    if ICEBERG_REST_TEST_S3_PATH_STYLE_ACCESS is not None:
        path_style_access = string_to_bool(ICEBERG_REST_TEST_S3_PATH_STYLE_ACCESS)
    else:
        path_style_access = None

    if ICEBERG_REST_TEST_S3_REGION is None:
        pytest.skip("ICEBERG_REST_TEST_S3_REGION is not set")

    print(f"path_style_access: {path_style_access}")

    return {
        "storage-profile": {
            "type": "s3",
            "bucket": ICEBERG_REST_TEST_S3_BUCKET,
            "region": ICEBERG_REST_TEST_S3_REGION,
            "path-style-access": path_style_access,
            "endpoint": ICEBERG_REST_TEST_S3_ENDPOINT,
        },
        "storage-credential": {
            "type": "s3",
            "credential-type": "access-key",
            "aws-access-key-id": ICEBERG_REST_TEST_S3_ACCESS_KEY,
            "aws-secret-access-key": ICEBERG_REST_TEST_S3_SECRET_KEY,
        },
    }


@dataclasses.dataclass
class Server:
    catalog_url: str
    management_url: str

    def create_warehouse(self, name: str, project_id: uuid.UUID) -> uuid.UUID:
        """Create a warehouse in this server"""
        storage_config = get_storage_config()

        create_payload = {
            "project-id": str(project_id),
            "warehouse-name": name,
            **storage_config,
        }

        warehouse_url = self.warehouse_url
        response = requests.post(warehouse_url, json=create_payload)
        if response.status_code != 200:
            raise ValueError(
                f"Failed to create warehouse ({response.status_code}): {response.text}"
            )

        warehouse_id = response.json()["warehouse-id"]
        return uuid.UUID(warehouse_id)

    @property
    def warehouse_url(self) -> str:
        return urllib.parse.urljoin(self.management_url, "v1/warehouse")


@dataclasses.dataclass
class Warehouse:
    server: Server
    project_id: uuid.UUID
    warehouse_id: uuid.UUID
    warehouse_name: str

    @property
    def pyiceberg_catalog(self) -> pyiceberg.catalog.rest.RestCatalog:
        return pyiceberg.catalog.rest.RestCatalog(
            name="my_catalog_name",
            uri=self.server.catalog_url,
            warehouse=f"{self.project_id}/{self.warehouse_name}",
            token="dummy",
        )


@dataclasses.dataclass
class Namespace:
    name: pyiceberg.typedef.Identifier
    warehouse: Warehouse

    @property
    def pyiceberg_catalog(self) -> pyiceberg.catalog.rest.RestCatalog:
        return self.warehouse.pyiceberg_catalog

    @property
    def spark_name(self) -> str:
        return "`" + ".".join(self.name) + "`"


@pytest.fixture(scope="session")
def server() -> Server:
    if ICEBERG_REST_TEST_MANAGEMENT_URL is None:
        pytest.skip("ICEBERG_REST_TEST_MANAGEMENT_URL is not set")
    if ICEBERG_REST_TEST_CATALOG_URL is None:
        pytest.skip("ICEBERG_REST_TEST_CATALOG_URL is not set")

    return Server(
        catalog_url=ICEBERG_REST_TEST_CATALOG_URL.rstrip("/") + "/",
        management_url=ICEBERG_REST_TEST_MANAGEMENT_URL.rstrip("/") + "/",
    )


@pytest.fixture(scope="session")
def warehouse(server: Server):
    project_id = uuid.uuid4()
    test_id = uuid.uuid4()
    warehouse_name = f"warehouse-{test_id}"
    warehouse_id = server.create_warehouse(warehouse_name, project_id=project_id)
    print(f"SERVER CREATED: {warehouse_id}")
    return Warehouse(
        server=server,
        project_id=project_id,
        warehouse_id=warehouse_id,
        warehouse_name=warehouse_name,
    )


@pytest.fixture(scope="function")
def namespace(warehouse: Warehouse):
    catalog = warehouse.pyiceberg_catalog
    namespace = (f"namespace-{uuid.uuid4()}",)
    catalog.create_namespace(namespace)
    return Namespace(name=namespace, warehouse=warehouse)


@pytest.fixture(scope="session")
def spark(warehouse: Warehouse):
    """Spark with a pre-configured Iceberg catalog"""
    try:
        import findspark

        findspark.init()
    except ImportError:
        pytest.skip("findspark not installed")

    import pyspark
    import pyspark.sql

    pyspark_version = pyspark.__version__
    # Strip patch version
    pyspark_version = ".".join(pyspark_version.split(".")[:2])

    spark_jars_packages = (
        f"org.apache.iceberg:iceberg-spark-runtime-{pyspark_version}_2.12:{ICEBERG_REST_TEST_SPARK_ICEBERG_VERSION},"
        f"org.apache.iceberg:iceberg-aws-bundle:{ICEBERG_REST_TEST_SPARK_ICEBERG_VERSION}"
    )
    configuration = {
        "spark.jars.packages": spark_jars_packages,
        "spark.sql.extensions": "org.apache.iceberg.spark.extensions.IcebergSparkSessionExtensions",
        "spark.sql.defaultCatalog": "test",
        f"spark.sql.catalog.test": "org.apache.iceberg.spark.SparkCatalog",
        f"spark.sql.catalog.test.catalog-impl": "org.apache.iceberg.rest.RESTCatalog",
        f"spark.sql.catalog.test.uri": warehouse.server.catalog_url,
        f"spark.sql.catalog.test.token": "dummy",
        f"spark.sql.catalog.test.warehouse": f"{warehouse.project_id}/{warehouse.warehouse_name}",
    }

    spark_conf = pyspark.SparkConf().setMaster("local[*]")

    for k, v in configuration.items():
        spark_conf = spark_conf.set(k, v)

    spark = pyspark.sql.SparkSession.builder.config(conf=spark_conf).getOrCreate()
    spark.sql(f"USE test")
    yield spark
    spark.stop()
