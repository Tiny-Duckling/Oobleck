from rest_framework import serializers
from .models import Opening, Application


class OpeningSerializer(serializers.ModelSerializer):
    created_by = serializers.StringRelatedField(default=serializers.CurrentUserDefault(), read_only=True)
    updated_by = serializers.StringRelatedField(default=serializers.CurrentUserDefault(), read_only=True)

    class Meta:
        model = Opening
        fields = '__all__'


class ApplicationSerializer(serializers.ModelSerializer):
    created_by = serializers.StringRelatedField(default=serializers.CurrentUserDefault(), read_only=True)
    updated_by = serializers.StringRelatedField(default=serializers.CurrentUserDefault(), read_only=True)

    class Meta:
        model = Application
        fields = '__all__'
